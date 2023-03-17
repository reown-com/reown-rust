use {
    self::connection::{connection_event_loop, ConnectionControl},
    crate::{ConnectionOptions, Error},
    relay_rpc::{
        domain::{SubscriptionId, Topic},
        rpc::{
            BatchFetch,
            BatchSubscribe,
            BatchUnsubscribe,
            Fetch,
            Publish,
            Subscribe,
            Subscription,
            Unsubscribe,
        },
    },
    std::{sync::Arc, time::Duration},
    tokio::sync::{
        mpsc::{self, UnboundedSender},
        oneshot,
    },
    tokio_tungstenite::tungstenite::protocol::CloseFrame,
};
pub use {fetch::*, inbound::*, outbound::*, stream::*};

mod connection;
mod fetch;
mod inbound;
mod outbound;
mod stream;

/// The message received from a subscription.
#[derive(Debug)]
pub struct PublishedMessage {
    pub topic: Topic,
    pub message: Arc<str>,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub received_at: chrono::DateTime<chrono::Utc>,
}

impl PublishedMessage {
    fn from_request(request: &InboundRequest<Subscription>) -> Self {
        let data = &request.data().data;
        let now = chrono::Utc::now();

        Self {
            topic: data.topic.clone(),
            message: data.message.clone(),
            // TODO: Set proper value once implemented.
            published_at: now,
            received_at: now,
        }
    }
}

/// Handlers for the RPC stream events.
pub trait ConnectionHandler: Send + 'static {
    /// Called when a connection to the Relay is established.
    fn connected(&mut self) {}

    /// Called when the Relay connection is closed.
    fn disconnected(&mut self, _frame: Option<CloseFrame<'static>>) {}

    /// Called when a message is received from the Relay.
    fn message_received(&mut self, message: PublishedMessage);

    /// Called when an inbound error occurs, such as data deserialization
    /// failure, or an unknown response message ID.
    fn inbound_error(&mut self, _error: Error) {}

    /// Called when an outbound error occurs, i.e. failed to write to the
    /// websocket stream.
    fn outbound_error(&mut self, _error: Error) {}
}

/// The Relay RPC client.
///
/// This provides the high-level access to all of the available RPC methods. For
/// a lower-level RPC stream see [`ClientStream`](crate::client::ClientStream).
#[derive(Debug, Clone)]
pub struct Client {
    control_tx: UnboundedSender<ConnectionControl>,
}

impl Client {
    /// Creates a new [`Client`] with the provided handler.
    pub fn new<T>(handler: T) -> Self
    where
        T: ConnectionHandler,
    {
        let (control_tx, control_rx) = mpsc::unbounded_channel();

        tokio::spawn(connection_event_loop(control_rx, handler));

        Self { control_tx }
    }

    /// Publishes a message over the network on given topic.
    pub fn publish(
        &self,
        topic: Topic,
        message: impl Into<Arc<str>>,
        tag: u32,
        ttl: Duration,
    ) -> EmptyResponseFuture<Publish> {
        let (request, response) = create_request(Publish {
            topic,
            message: message.into(),
            ttl_secs: ttl.as_secs() as u32,
            tag,
            prompt: false,
        });

        self.request(request);

        EmptyResponseFuture::new(response)
    }

    /// Subscribes on topic to receive messages.
    pub fn subscribe(&self, topic: Topic) -> ResponseFuture<Subscribe> {
        let (request, response) = create_request(Subscribe { topic });

        self.request(request);

        response
    }

    /// Unsubscribes from a topic.
    pub fn unsubscribe(
        &self,
        topic: Topic,
        subscription_id: SubscriptionId,
    ) -> EmptyResponseFuture<Unsubscribe> {
        let (request, response) = create_request(Unsubscribe {
            topic,
            subscription_id,
        });

        self.request(request);

        EmptyResponseFuture::new(response)
    }

    /// Fetch mailbox messages for a specific topic.
    pub fn fetch(&self, topic: Topic) -> ResponseFuture<Fetch> {
        let (request, response) = create_request(Fetch { topic });

        self.request(request);

        response
    }

    /// Fetch mailbox messages for a specific topic. Returns a [`Stream`].
    pub fn fetch_stream(&self, topics: impl Into<Vec<Topic>>) -> FetchMessageStream {
        FetchMessageStream::new(self.clone(), topics.into())
    }

    /// Subscribes on multiple topics to receive messages.
    pub fn batch_subscribe(&self, topics: impl Into<Vec<Topic>>) -> ResponseFuture<BatchSubscribe> {
        let (request, response) = create_request(BatchSubscribe {
            topics: topics.into(),
        });

        self.request(request);

        response
    }

    /// Unsubscribes from multiple topics.
    pub fn batch_unsubscribe(
        &self,
        subscriptions: impl Into<Vec<Unsubscribe>>,
    ) -> EmptyResponseFuture<BatchUnsubscribe> {
        let (request, response) = create_request(BatchUnsubscribe {
            subscriptions: subscriptions.into(),
        });

        self.request(request);

        EmptyResponseFuture::new(response)
    }

    /// Fetch mailbox messages for multiple topics.
    pub fn batch_fetch(&self, topics: impl Into<Vec<Topic>>) -> ResponseFuture<BatchFetch> {
        let (request, response) = create_request(BatchFetch {
            topics: topics.into(),
        });

        self.request(request);

        response
    }

    /// Opens a connection to the Relay.
    pub async fn connect(&self, opts: ConnectionOptions) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();
        let opts = Box::new(opts);

        if self
            .control_tx
            .send(ConnectionControl::Connect { opts, tx })
            .is_ok()
        {
            rx.await.map_err(|_| Error::ChannelClosed)?
        } else {
            Err(Error::ChannelClosed)
        }
    }

    /// Closes the Relay connection.
    pub async fn disconnect(&self) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();

        if self
            .control_tx
            .send(ConnectionControl::Disconnect { tx })
            .is_ok()
        {
            rx.await.map_err(|_| Error::ChannelClosed)?
        } else {
            Err(Error::ChannelClosed)
        }
    }

    pub(crate) fn request(&self, request: OutboundRequest) {
        if let Err(err) = self
            .control_tx
            .send(ConnectionControl::OutboundRequest(request))
        {
            let ConnectionControl::OutboundRequest(request) = err.0 else {
                unreachable!();
            };

            request.tx.send(Err(Error::ChannelClosed)).ok();
        }
    }
}
