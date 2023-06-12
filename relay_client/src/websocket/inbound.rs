use {
    crate::Error,
    relay_rpc::{
        domain::MessageId,
        rpc::{ErrorResponse, Payload, RequestPayload, Response, SuccessfulResponse},
    },
    tokio::sync::mpsc::UnboundedSender,
    tokio_tungstenite::tungstenite::Message,
};

/// The lower-level inbound RPC request.
///
/// Provides access to the request payload (via [`InboundRequest::data()`]) and
/// the response channel (via [`InboundRequest::respond()`]).
///
/// Currently, the only inbound RPC request the client can receive is
/// [`Subscription`][relay_rpc::rpc::Subscription].
#[derive(Debug)]
pub struct InboundRequest<T> {
    id: MessageId,
    tx: UnboundedSender<Message>,
    data: T,
}

impl<T> InboundRequest<T>
where
    T: RequestPayload,
{
    pub(super) fn new(id: MessageId, data: T, tx: UnboundedSender<Message>) -> Self {
        Self { id, tx, data }
    }

    /// Returns the request payload reference.
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Sends the response back to the Relay. The value is a
    /// `Result<T::Response, T::Error>` (see [`RequestPayload`] trait for
    /// details).
    ///
    /// Returns an error if the response can't be serialized, or if the
    /// underlying channel is closed.
    pub fn respond(self, response: Result<T::Response, T::Error>) -> Result<(), Error> {
        let response = match response {
            Ok(data) => Response::Success(SuccessfulResponse::new(
                self.id,
                serde_json::to_value(data).map_err(Error::Serialization)?,
            )),

            Err(err) => Response::Error(ErrorResponse::new(self.id, err.into())),
        };

        let message = Message::Text(
            serde_json::to_string(&Payload::Response(response)).map_err(Error::Serialization)?,
        );

        self.tx.send(message).map_err(|_| Error::ChannelClosed)
    }
}
