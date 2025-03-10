#[cfg(not(target_arch = "wasm32"))]
use tokio_tungstenite::{
    connect_async,
    tungstenite::{protocol::CloseFrame, Message},
    MaybeTlsStream,
    WebSocketStream,
};
#[cfg(target_arch = "wasm32")]
use tokio_tungstenite_wasm::{connect as connect_async, CloseFrame, Message, WebSocketStream};
use {
    super::{
        inbound::InboundRequest,
        outbound::{create_request, OutboundRequest, ResponseFuture},
        CloseReason,
        TransportError,
        WebsocketClientError,
    },
    crate::{error::ClientError, HttpRequest, MessageIdGenerator},
    futures_util::{stream::FusedStream, SinkExt, Stream, StreamExt},
    relay_rpc::{
        domain::MessageId,
        rpc::{self, Params, Payload, Response, ServiceRequest, Subscription},
    },
    std::{
        collections::{hash_map::Entry, HashMap},
        pin::Pin,
        task::{Context, Poll},
    },
    tokio::sync::{
        mpsc,
        mpsc::{UnboundedReceiver, UnboundedSender},
        oneshot,
    },
};
#[cfg(not(target_arch = "wasm32"))]
pub type SocketStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
#[cfg(not(target_arch = "wasm32"))]
use tokio::net::TcpStream;
#[cfg(target_arch = "wasm32")]
pub type SocketStream = WebSocketStream;

/// Opens a connection to the Relay and returns [`ClientStream`] for the
/// connection.
#[cfg(not(target_arch = "wasm32"))]
pub async fn create_stream(request: HttpRequest<()>) -> Result<ClientStream, WebsocketClientError> {
    let (socket, _) = connect_async(request)
        .await
        .map_err(WebsocketClientError::ConnectionFailed)?;

    Ok(ClientStream::new(socket))
}

#[cfg(target_arch = "wasm32")]
pub async fn create_stream(request: HttpRequest<()>) -> Result<ClientStream, WebsocketClientError> {
    let url = format!("{}", request.uri());
    let socket = connect_async(url)
        .await
        .map_err(WebsocketClientError::ConnectionFailed)?;

    Ok(ClientStream::new(socket))
}

/// Possible events produced by the [`ClientStream`].
///
/// The events are produced by polling [`ClientStream`] in a loop.
#[derive(Debug)]
pub enum StreamEvent {
    /// Inbound request for receiving a subscription message.
    ///
    /// Currently, [`Subscription`] is the only request that the Relay sends to
    /// the clients.
    InboundSubscriptionRequest(InboundRequest<Subscription>),

    /// Error generated when failed to parse an inbound message, invalid request
    /// type or message ID.
    InboundError(ClientError),

    /// Error generated when failed to write data to the underlying websocket
    /// stream.
    OutboundError(ClientError),

    /// The websocket connection was closed.
    ///
    /// This is the last event that can be produced by the stream.
    ConnectionClosed(Option<CloseFrame<'static>>),
}

/// Lower-level [`FusedStream`] interface for the client connection.
///
/// The stream produces [`StreamEvent`] when polled, and can be used to send RPC
/// requests (see [`ClientStream::send()`] and [`ClientStream::send_raw()`]).
///
/// For a higher-level interface see [`Client`](crate::client::Client). For an
/// example usage of the stream see `client::connection` module.
pub struct ClientStream {
    socket: SocketStream,
    outbound_tx: UnboundedSender<Message>,
    outbound_rx: UnboundedReceiver<Message>,
    requests: HashMap<MessageId, oneshot::Sender<Result<serde_json::Value, ClientError>>>,
    id_generator: MessageIdGenerator,
    close_frame: Option<CloseFrame<'static>>,
}

impl ClientStream {
    pub fn new(socket: SocketStream) -> Self {
        let requests = HashMap::new();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        let id_generator = MessageIdGenerator::new();

        Self {
            socket,
            outbound_tx,
            outbound_rx,
            requests,
            id_generator,
            close_frame: None,
        }
    }

    /// Sends an already serialized [`OutboundRequest`][OutboundRequest] (see
    /// [`create_request()`]).
    pub fn send_raw(&mut self, request: OutboundRequest) {
        let tx = request.tx;
        let id = self.id_generator.next();
        let request = Payload::Request(rpc::Request::new(id, request.params));
        let serialized = serde_json::to_string(&request);

        match serialized {
            Ok(data) => match self.requests.entry(id) {
                Entry::Occupied(_) => {
                    tx.send(Err(ClientError::DuplicateRequestId)).ok();
                }

                Entry::Vacant(entry) => {
                    entry.insert(tx);
                    self.outbound_tx.send(Message::Text(data)).ok();
                }
            },

            Err(err) => {
                tx.send(Err(ClientError::Serialization(err))).ok();
            }
        }
    }

    /// Serialize the request into a generic [`OutboundRequest`] and sends it,
    /// returning a future that resolves with the response.
    pub fn send<T>(&mut self, request: T) -> ResponseFuture<T>
    where
        T: ServiceRequest,
    {
        let (request, response) = create_request(request);
        self.send_raw(request);
        response
    }

    /// Closes the connection.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn close(&mut self, frame: Option<CloseFrame<'static>>) -> Result<(), ClientError> {
        self.close_frame = frame.clone();
        self.socket
            .close(frame)
            .await
            .map_err(|err| WebsocketClientError::ClosingFailed(err).into())
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn close(&mut self, frame: Option<CloseFrame<'static>>) -> Result<(), ClientError> {
        self.close_frame = frame.clone();
        self.socket
            .close()
            .await
            .map_err(|err| WebsocketClientError::ClosingFailed(err).into())
    }

    fn parse_inbound(&mut self, result: Result<Message, TransportError>) -> Option<StreamEvent> {
        match result {
            Ok(message) => match &message {
                Message::Binary(_) | Message::Text(_) => {
                    let payload: Payload = match serde_json::from_slice(&message.into_data()) {
                        Ok(payload) => payload,

                        Err(err) => {
                            return Some(StreamEvent::InboundError(ClientError::Deserialization(
                                err,
                            )))
                        }
                    };

                    match payload {
                        Payload::Request(request) => {
                            let id = request.id;

                            let event =
                                match request.params {
                                    Params::Subscription(data) => {
                                        StreamEvent::InboundSubscriptionRequest(
                                            InboundRequest::new(id, data, self.outbound_tx.clone()),
                                        )
                                    }

                                    _ => StreamEvent::InboundError(ClientError::InvalidRequestType),
                                };

                            Some(event)
                        }

                        Payload::Response(response) => {
                            let id = response.id();

                            if id.is_zero() {
                                return match response {
                                    Response::Error(response) => Some(StreamEvent::InboundError(
                                        ClientError::from(response.error),
                                    )),

                                    Response::Success(_) => Some(StreamEvent::InboundError(
                                        ClientError::InvalidResponseId,
                                    )),
                                };
                            }

                            if let Some(tx) = self.requests.remove(&id) {
                                let result = match response {
                                    Response::Error(response) => {
                                        Err(ClientError::from(response.error))
                                    }

                                    Response::Success(response) => Ok(response.result),
                                };

                                tx.send(result).ok();

                                // Perform compaction if required.
                                if self.requests.len() * 3 < self.requests.capacity() {
                                    self.requests.shrink_to_fit();
                                }

                                None
                            } else {
                                Some(StreamEvent::InboundError(ClientError::InvalidResponseId))
                            }
                        }
                    }
                }

                Message::Close(frame) => {
                    self.close_frame = frame.clone();
                    Some(StreamEvent::ConnectionClosed(frame.clone()))
                }
                #[cfg(not(target_arch = "wasm32"))]
                _ => None,
            },

            Err(error) => Some(StreamEvent::InboundError(
                WebsocketClientError::Transport(error).into(),
            )),
        }
    }

    fn poll_write(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), TransportError>> {
        let mut should_flush = false;

        loop {
            // `poll_ready() needs to be called before each `start_send()` to make sure the
            // sink is ready to accept more data.
            match self.socket.poll_ready_unpin(cx) {
                // The sink is ready to accept more data.
                Poll::Ready(Ok(())) => {
                    if let Poll::Ready(Some(next_message)) = self.outbound_rx.poll_recv(cx) {
                        if let Err(err) = self.socket.start_send_unpin(next_message) {
                            return Poll::Ready(Err(err));
                        }

                        should_flush = true;
                    } else if should_flush {
                        // We've sent out some messages, now we need to flush.
                        return self.socket.poll_flush_unpin(cx);
                    } else {
                        return Poll::Pending;
                    }
                }

                Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),

                // The sink is not ready.
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl Stream for ClientStream {
    type Item = StreamEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.socket.is_terminated() {
            return Poll::Ready(None);
        }

        while let Poll::Ready(data) = self.socket.poll_next_unpin(cx) {
            match data {
                Some(result) => {
                    if let Some(event) = self.parse_inbound(result) {
                        return Poll::Ready(Some(event));
                    }
                }

                None => {
                    return Poll::Ready(Some(StreamEvent::ConnectionClosed(
                        self.close_frame.clone(),
                    )))
                }
            }
        }

        match self.poll_write(cx) {
            Poll::Ready(Err(error)) => Poll::Ready(Some(StreamEvent::OutboundError(
                WebsocketClientError::Transport(error).into(),
            ))),

            _ => Poll::Pending,
        }
    }
}

impl FusedStream for ClientStream {
    #[cfg(not(target_arch = "wasm32"))]
    fn is_terminated(&self) -> bool {
        self.socket.is_terminated()
    }

    #[cfg(target_arch = "wasm32")]
    fn is_terminated(&self) -> bool {
        false
    }
}

impl Drop for ClientStream {
    fn drop(&mut self) {
        let reason = CloseReason(self.close_frame.take());

        for (_, tx) in self.requests.drain() {
            tx.send(Err(
                WebsocketClientError::ConnectionClosed(reason.clone()).into()
            ))
            .ok();
        }
    }
}
