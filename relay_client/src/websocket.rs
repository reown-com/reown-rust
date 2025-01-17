use {
    self::connection::{connection_event_loop, ConnectionControl},
    crate::{
        error::{ClientError, Error},
        ConnectionOptions,
    },
    relay_rpc::{
        domain::{MessageId, SubscriptionId, Topic},
        rpc::{
            BatchFetchMessages,
            BatchReceiveMessages,
            BatchSubscribe,
            BatchSubscribeBlocking,
            BatchUnsubscribe,
            FetchMessages,
            Publish,
            Receipt,
            Subscribe,
            SubscribeBlocking,
            Subscription,
            SubscriptionError,
            Unsubscribe,
        },
    },
    std::{future::Future, sync::Arc, time::Duration},
    tokio::sync::{
        mpsc::{self, UnboundedSender},
        oneshot,
    },
};
pub use {
    fetch::*,
    inbound::*,
    outbound::*,
    stream::*,
    tokio_tungstenite::tungstenite::protocol::CloseFrame,
};

pub type TransportError = tokio_tungstenite::tungstenite::Error;

#[derive(Debug, thiserror::Error)]
pub enum WebsocketClientError {
    #[error("Failed to connect: {0}")]
    ConnectionFailed(TransportError),

    #[error("Connection closed: {0}")]
    ConnectionClosed(CloseReason),

    #[error("Failed to close connection: {0}")]
    ClosingFailed(TransportError),

    #[error("Websocket transport error: {0}")]
    Transport(TransportError),

    #[error("Not connected")]
    NotConnected,
}

/// Wrapper around the websocket [`CloseFrame`] providing info about the
/// connection closing reason.
#[derive(Debug, Clone)]
pub struct CloseReason(pub Option<CloseFrame<'static>>);

impl std::fmt::Display for CloseReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(frame) = &self.0 {
            frame.fmt(f)
        } else {
            f.write_str("<close frame unavailable>")
        }
    }
}

mod connection;
mod fetch;
mod inbound;
mod outbound;
mod stream;

/// The message received from a subscription.
#[derive(Debug, Clone)]
pub struct PublishedMessage {
    pub message_id: MessageId,
    pub subscription_id: SubscriptionId,
    pub topic: Topic,
    pub message: Arc<str>,
    pub tag: u32,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub received_at: chrono::DateTime<chrono::Utc>,
}

impl PublishedMessage {
    fn from_request(request: &InboundRequest<Subscription>) -> Self {
        let Subscription { id, data } = request.data();
        let now = chrono::Utc::now();

        Self {
            message_id: request.id(),
            subscription_id: id.clone(),
            topic: data.topic.clone(),
            message: data.message.clone(),
            tag: data.tag,
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
    fn inbound_error(&mut self, _error: ClientError) {}

    /// Called when an outbound error occurs, i.e. failed to write to the
    /// websocket stream.
    fn outbound_error(&mut self, _error: ClientError) {}
}

type SubscriptionResult<T> = Result<T, Error<SubscriptionError>>;

/// The Relay WebSocket RPC client.
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
        attestation: impl Into<Option<Arc<str>>>,
        tag: u32,
        ttl: Duration,
        prompt: bool,
    ) -> EmptyResponseFuture<Publish> {
        let (request, response) = create_request(Publish {
            topic,
            message: message.into(),
            attestation: attestation.into(),
            ttl_secs: ttl.as_secs() as u32,
            tag,
            prompt,
            tvf_data: None,
        });

        self.request(request);

        EmptyResponseFuture::new(response)
    }

    /// Subscribes on topic to receive messages. The request is resolved
    /// optimistically as soon as the relay receives it.
    pub fn subscribe(&self, topic: Topic) -> ResponseFuture<Subscribe> {
        let (request, response) = create_request(Subscribe { topic });

        self.request(request);

        response
    }

    /// Subscribes on topic to receive messages. The request is resolved only
    /// when fully processed by the relay.
    /// Note: This function is experimental and will likely be removed in the
    /// future.
    pub fn subscribe_blocking(&self, topic: Topic) -> ResponseFuture<SubscribeBlocking> {
        let (request, response) = create_request(SubscribeBlocking { topic });

        self.request(request);

        response
    }

    /// Unsubscribes from a topic.
    pub fn unsubscribe(&self, topic: Topic) -> EmptyResponseFuture<Unsubscribe> {
        let (request, response) = create_request(Unsubscribe { topic });

        self.request(request);

        EmptyResponseFuture::new(response)
    }

    /// Fetch mailbox messages for a specific topic.
    pub fn fetch(&self, topic: Topic) -> ResponseFuture<FetchMessages> {
        let (request, response) = create_request(FetchMessages { topic });

        self.request(request);

        response
    }

    /// Fetch mailbox messages for a specific topic. Returns a [`Stream`].
    pub fn fetch_stream(&self, topics: impl Into<Vec<Topic>>) -> FetchMessageStream {
        FetchMessageStream::new(self.clone(), topics.into())
    }

    /// Subscribes on multiple topics to receive messages. The request is
    /// resolved optimistically as soon as the relay receives it.
    pub fn batch_subscribe(&self, topics: impl Into<Vec<Topic>>) -> ResponseFuture<BatchSubscribe> {
        let (request, response) = create_request(BatchSubscribe {
            topics: topics.into(),
        });

        self.request(request);

        response
    }

    /// Subscribes on multiple topics to receive messages. The request is
    /// resolved only when fully processed by the relay.
    /// Note: This function is experimental and will likely be removed in the
    /// future.
    pub fn batch_subscribe_blocking(
        &self,
        topics: impl Into<Vec<Topic>>,
    ) -> impl Future<Output = SubscriptionResult<Vec<SubscriptionResult<SubscriptionId>>>> {
        let (request, response) = create_request(BatchSubscribeBlocking {
            topics: topics.into(),
        });

        self.request(request);

        async move {
            Ok(response
                .await?
                .into_iter()
                .map(crate::convert_subscription_result)
                .collect())
        }
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
    pub fn batch_fetch(&self, topics: impl Into<Vec<Topic>>) -> ResponseFuture<BatchFetchMessages> {
        let (request, response) = create_request(BatchFetchMessages {
            topics: topics.into(),
        });

        self.request(request);

        response
    }

    /// Acknowledge receipt of messages from a subscribed client.
    pub async fn batch_receive(
        &self,
        receipts: impl Into<Vec<Receipt>>,
    ) -> ResponseFuture<BatchReceiveMessages> {
        let (request, response) = create_request(BatchReceiveMessages {
            receipts: receipts.into(),
        });

        self.request(request);

        response
    }

    /// Opens a connection to the Relay.
    pub async fn connect(&self, opts: &ConnectionOptions) -> Result<(), ClientError> {
        let (tx, rx) = oneshot::channel();
        let request = opts.as_ws_request()?;

        if self
            .control_tx
            .send(ConnectionControl::Connect { request, tx })
            .is_ok()
        {
            rx.await.map_err(|_| ClientError::ChannelClosed)?
        } else {
            Err(ClientError::ChannelClosed)
        }
    }

    /// Closes the Relay connection.
    pub async fn disconnect(&self) -> Result<(), ClientError> {
        let (tx, rx) = oneshot::channel();

        if self
            .control_tx
            .send(ConnectionControl::Disconnect { tx })
            .is_ok()
        {
            rx.await.map_err(|_| ClientError::ChannelClosed)?
        } else {
            Err(ClientError::ChannelClosed)
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

            request.tx.send(Err(ClientError::ChannelClosed)).ok();
        }
    }
}
