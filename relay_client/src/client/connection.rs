use {
    super::{
        outbound::OutboundRequest,
        stream::{create_stream, ClientStream},
    },
    crate::{
        client::stream::StreamEvent,
        ConnectionHandler,
        ConnectionOptions,
        Error,
        PublishedMessage,
        WsError,
    },
    futures_util::{stream::FusedStream, Stream, StreamExt},
    std::{
        pin::Pin,
        task::{Context, Poll},
    },
    tokio::sync::{mpsc::UnboundedReceiver, oneshot},
};

pub(super) enum ConnectionControl {
    Connect {
        opts: Box<ConnectionOptions>,
        tx: oneshot::Sender<Result<(), Error>>,
    },

    Disconnect {
        tx: oneshot::Sender<Result<(), Error>>,
    },

    OutboundRequest(OutboundRequest),
}

pub(super) async fn connection_event_loop<T>(
    mut control_rx: UnboundedReceiver<ConnectionControl>,
    mut handler: T,
) where
    T: ConnectionHandler,
{
    let mut conn = Connection::new();

    loop {
        tokio::select! {
            event = control_rx.recv() => {
                match event {
                    Some(event) => match event {
                        ConnectionControl::Connect { tx, opts } => {
                            let result = conn.connect(*opts).await;

                            if result.is_ok() {
                                handler.connected();
                            }

                            tx.send(result).ok();
                        }

                        ConnectionControl::Disconnect { tx } => {
                            tx.send(conn.disconnect().await).ok();
                        }

                        ConnectionControl::OutboundRequest(request) => {
                            conn.request(request);
                        }
                    }

                    // Control TX has been dropped, shutting down.
                    None => {
                        conn.disconnect().await.ok();
                        handler.disconnected(None);
                        break;
                    }
                }
            }

            event = conn.select_next_some() => {
                match event {
                    StreamEvent::InboundSubscriptionRequest(request) => {
                        handler.message_received(PublishedMessage::from_request(&request));
                        request.respond(Ok(true)).ok();
                    }

                    StreamEvent::InboundError(error) => {
                        handler.inbound_error(error);
                    }

                    StreamEvent::OutboundError(error) => {
                        handler.outbound_error(error);
                    }

                    StreamEvent::ConnectionClosed(frame) => {
                        handler.disconnected(frame);
                        conn.reset();
                    }
                }
            }
        }
    }
}

struct Connection {
    stream: Option<ClientStream>,
}

impl Connection {
    fn new() -> Self {
        Self { stream: None }
    }

    async fn connect(&mut self, opts: ConnectionOptions) -> Result<(), Error> {
        if let Some(mut stream) = self.stream.take() {
            stream.close(None).await?;
        }

        self.stream = Some(create_stream(opts).await?);

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Error> {
        let stream = self.stream.take();

        match stream {
            Some(mut stream) => stream.close(None).await,

            None => Err(Error::ClosingFailed(WsError::AlreadyClosed)),
        }
    }

    fn request(&mut self, request: OutboundRequest) {
        match &mut self.stream {
            Some(stream) => stream.send_raw(request),

            None => {
                request.tx.send(Err(Error::NotConnected)).ok();
            }
        }
    }

    fn reset(&mut self) {
        self.stream = None;
    }
}

impl Stream for Connection {
    type Item = StreamEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(stream) = &mut self.stream {
            if stream.is_terminated() {
                self.stream = None;

                Poll::Pending
            } else {
                stream.poll_next_unpin(cx)
            }
        } else {
            Poll::Pending
        }
    }
}

impl FusedStream for Connection {
    fn is_terminated(&self) -> bool {
        false
    }
}
