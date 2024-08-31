use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::Level;

use crate::*;

use self::agiparse::{AGIMessage, AGIParseError, AGIStatusGeneric};
use self::command::{AGICommand, AGIResponse};

#[derive(Debug)]
pub struct Connection {
    buf: [u8; 1024],
    stream: TcpStream,
}
impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            buf: [0; 1024],
            stream,
        }
    }

    /// Send an AGI Command over this connection
    ///
    /// Return an Error when sending fails or we do not get a Status message as a response.
    /// non-200 status codes are returned as Ok(the-status) and are NOT an Err as far as this
    /// method is concerned.
    #[tracing::instrument(level=Level::TRACE, ret, err)]
    pub async fn send_command<H>(&mut self, command: H) -> Result<AGIResponse<H::Response>, AGIError>
        where H: AGICommand
    {
        let string_to_send = command.to_string();
        // send the command over the stream
        self.stream
            .write(string_to_send.as_bytes())
            .await
            .map_err(|e| AGIError::CannotSendCommand(e))?;
        // make sure that we get an AGIStatus as a result
        let response = self
            .read_and_parse()
            .await
            .map_err(|e| AGIError::ParseError(e))?;
        // Get the response and return it
        let status = match response {
            AGIMessage::Status(x) => Ok(x),
            x => Err(AGIError::NotAStatus(x)),
        }?;
        match status {
            AGIStatusGeneric::Ok(ref result, ref op_data) => {
                let status_specialized = H::Response::try_from((result, op_data.as_deref()))
                    .map_err(|e| AGIError::AGIStatusUnspecializable(status, e.response_to_command))?;
                Ok(AGIResponse::Ok(status_specialized))
            },
            AGIStatusGeneric::Invalid => Ok(AGIResponse::Invalid),
            AGIStatusGeneric::DeadChannel => Ok(AGIResponse::DeadChannel),
            AGIStatusGeneric::EndUsage => Ok(AGIResponse::EndUsage),
        }
    }

    /// Read the next packet and parse it as an AGIMessage
    #[tracing::instrument(level=Level::TRACE, ret, err)]
    pub(crate) async fn read_and_parse(&mut self) -> Result<AGIMessage, AGIParseError> {
        let num_read = self.stream.read(&mut self.buf).await.unwrap();
        if num_read == 0 {
            return Err(AGIParseError::NoBytes);
        };

        match std::str::from_utf8(&self.buf) {
            Err(_) => Err(AGIParseError::NotUtf8),
            // and it needs to be parsable as an AGI message
            Ok(x) => x.parse::<AGIMessage>(),
        }
    }
}
