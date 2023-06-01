use {
    crate::Error,
    pin_project::pin_project,
    relay_rpc::rpc::{Params, RequestPayload},
    std::{
        future::Future,
        marker::PhantomData,
        pin::Pin,
        task::{ready, Context, Poll},
    },
    tokio::sync::oneshot,
};

/// An outbound request wrapper created by [`create_request()`]. Intended be
/// used with [`ClientStream`][crate::client::ClientStream].
#[derive(Debug)]
pub struct OutboundRequest {
    pub(super) params: Params,
    pub(super) tx: oneshot::Sender<Result<serde_json::Value, Error>>,
}

impl OutboundRequest {
    pub(super) fn new(
        params: Params,
        tx: oneshot::Sender<Result<serde_json::Value, Error>>,
    ) -> Self {
        Self { params, tx }
    }
}

/// Future that resolves with the RPC response for the specified request.
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
pub struct ResponseFuture<T> {
    #[pin]
    rx: oneshot::Receiver<Result<serde_json::Value, Error>>,
    _marker: PhantomData<T>,
}

impl<T> ResponseFuture<T> {
    pub(super) fn new(rx: oneshot::Receiver<Result<serde_json::Value, Error>>) -> Self {
        Self {
            rx,
            _marker: PhantomData,
        }
    }
}

impl<T> Future for ResponseFuture<T>
where
    T: RequestPayload,
{
    type Output = Result<T::Response, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let result = ready!(this.rx.poll(cx)).map_err(|_| Error::ChannelClosed)?;

        let result = match result {
            Ok(value) => serde_json::from_value(value).map_err(Error::Deserialization),

            Err(err) => Err(err),
        };

        Poll::Ready(result)
    }
}

/// Future that resolves with the RPC response, consuming it and returning
/// `Result<(), Error>`.
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
pub struct EmptyResponseFuture<T> {
    #[pin]
    rx: ResponseFuture<T>,
}

impl<T> EmptyResponseFuture<T> {
    pub(super) fn new(rx: ResponseFuture<T>) -> Self {
        Self { rx }
    }
}

impl<T> Future for EmptyResponseFuture<T>
where
    T: RequestPayload,
{
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(ready!(self.project().rx.poll(cx)).map(|_| ()))
    }
}

/// Creates an RPC request and returns a tuple of the request and a response
/// future. The request is intended to be used with
/// [`ClientStream`][crate::client::ClientStream].
pub fn create_request<T>(data: T) -> (OutboundRequest, ResponseFuture<T>)
where
    T: RequestPayload,
{
    let (tx, rx) = oneshot::channel();

    (
        OutboundRequest::new(data.into_params(), tx),
        ResponseFuture::new(rx),
    )
}
