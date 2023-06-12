use {
    super::{create_request, Client, ResponseFuture},
    crate::Error,
    futures_util::{FutureExt, Stream},
    relay_rpc::{
        domain::Topic,
        rpc::{BatchFetchMessages, SubscriptionData},
    },
    std::{
        pin::Pin,
        task::{Context, Poll},
    },
};

/// Stream that uses the `irn_batchFetch` RPC method to retrieve messages from
/// the Relay.
pub struct FetchMessageStream {
    client: Client,
    request: BatchFetchMessages,
    batch: Option<std::vec::IntoIter<SubscriptionData>>,
    batch_fut: Option<ResponseFuture<BatchFetchMessages>>,
    has_more: bool,
}

impl FetchMessageStream {
    pub(super) fn new(client: Client, topics: impl Into<Vec<Topic>>) -> Self {
        let request = BatchFetchMessages {
            topics: topics.into(),
        };

        Self {
            client,
            request,
            batch: None,
            batch_fut: None,
            has_more: true,
        }
    }

    /// Clears all internal state so that on the next stream poll it returns
    /// `None` and finishes data streaming.
    #[inline]
    fn clear(&mut self) {
        self.batch = None;
        self.batch_fut = None;
        self.has_more = false;
    }
}

impl Stream for FetchMessageStream {
    type Item = Result<SubscriptionData, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if let Some(batch) = &mut self.batch {
                // Drain the items from the batch, if we have one.
                match batch.next() {
                    Some(data) => {
                        return Poll::Ready(Some(Ok(data)));
                    }

                    None => {
                        // No more items in the batch, fetch the next batch.
                        self.batch = None;
                    }
                }
            } else if let Some(batch_fut) = &mut self.batch_fut {
                // Waiting for the next batch to arrive.
                match batch_fut.poll_unpin(cx) {
                    // The next batch is ready. Update `has_more` flag and clear the batch future.
                    Poll::Ready(Ok(response)) => {
                        self.batch = Some(response.messages.into_iter());
                        self.batch_fut = None;
                        self.has_more = response.has_more;
                    }

                    // Error receiving the next batch. This is unrecoverable, so clear the state and
                    // end the stream.
                    Poll::Ready(Err(err)) => {
                        self.clear();

                        return Poll::Ready(Some(Err(err)));
                    }

                    // The batch is not ready yet.
                    Poll::Pending => {
                        return Poll::Pending;
                    }
                };
            } else if self.has_more {
                // We have neither a batch, or a batch future, but `has_more` flag is set. Set
                // up a future to receive the next batch.
                let (request, batch_fut) = create_request(self.request.clone());

                self.client.request(request);
                self.batch_fut = Some(batch_fut);
            } else {
                // The stream can't produce any more items, since it doesn't have neither a
                // batch of data or a future for receiving the next batch, and `has_more` flag
                // is not set.
                return Poll::Ready(None);
            }
        }
    }
}
