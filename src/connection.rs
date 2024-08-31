//! This module handles the literal network connection and sends/receives packets.
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::Level;

use crate::*;

use self::agiparse::{AGIMessage, AGIParseError, AGIStatusGeneric};
use crate::command::{AGICommand, AGIResponse};

/// `Connection` handles a single AGI stream (a connection originating from a client).
/// [`command`]s are sent with [`connection::Connection::send_command`](self::Connection::send_command)
#[derive(Debug)]
pub struct Connection {
    buf: [u8; 1024],
    stream: TcpStream,
}
impl Connection {
    pub(crate) fn new(stream: TcpStream) -> Connection {
        Connection {
            buf: [0; 1024],
            stream,
        }
    }

    /// Send an AGI Command over this connection.
    ///
    /// Return an Error when sending fails or we do not get a Status message as a response.
    /// non-200 status codes are returned as Ok(the-status) and are NOT an Err as far as this
    /// method is concerned.
    ///
    /// Note that the precice return type depends on the command sent.
    #[tracing::instrument(level=Level::TRACE, ret, err)]
    pub async fn send_command<H>(
        &mut self,
        command: H,
    ) -> Result<AGIResponse<H::Response>, AGIError>
    where
        H: AGICommand,
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
        Self::agi_response_as_specialized_status::<H>(response)
    }

    /// Parse an AGI message, assuming that is is a response to Command `H`.
    fn agi_response_as_specialized_status<H>(
        message: AGIMessage,
    ) -> Result<AGIResponse<H::Response>, AGIError>
    where
        H: AGICommand,
    {
        // Get the response and return it
        let status = match message {
            AGIMessage::Status(x) => Ok(x),
            x => Err(AGIError::NotAStatus(x)),
        }?;
        match status {
            AGIStatusGeneric::Ok(ref result, ref op_data) => {
                let status_specialized = H::Response::try_from((result, op_data.as_deref()))
                    .map_err(|e| {
                        AGIError::AGIStatusUnspecializable(status, e.response_to_command)
                    })?;
                Ok(AGIResponse::Ok(status_specialized))
            }
            AGIStatusGeneric::Invalid => Ok(AGIResponse::Invalid),
            AGIStatusGeneric::DeadChannel => Ok(AGIResponse::DeadChannel),
            AGIStatusGeneric::EndUsage => Ok(AGIResponse::EndUsage),
        }
    }

    /// Read the next packet and parse it as an AGIMessage
    #[tracing::instrument(level=Level::TRACE, ret, err)]
    pub(crate) async fn read_and_parse(&mut self) -> Result<AGIMessage, AGIParseError> {
        let num_read = self.stream.read(&mut self.buf).await.unwrap();
        // empty packets are not accepted
        if num_read == 0 {
            return Err(AGIParseError::NoBytes);
        };

        match std::str::from_utf8(&self.buf) {
            // the packet needs to be utf8
            Err(_) => Err(AGIParseError::NotUtf8),
            // and it needs to be parsable as an AGI message
            Ok(x) => x.parse::<AGIMessage>(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::command::{
        answer::{Answer, AnswerResponse}, get_full_variable::{GetFullVariable, ThisChannel}, raw_command::RawCommandResponse, verbose::Verbose, RawCommand, SetVariable
    };

    use super::*;

    #[test]
    fn parse_answer_response() {
        let response_body = AGIMessage::Status(AGIStatusGeneric::Ok(
            "-1".to_owned(),
            Some("did not work".to_owned()),
        ));
        assert_eq!(
            Connection::agi_response_as_specialized_status::<Answer>(response_body).unwrap(),
            AGIResponse::Ok(AnswerResponse::Failure)
        );
    }

    #[test]
    fn parse_verbose_response() {
        let response_body =
            AGIMessage::Status(AGIStatusGeneric::Ok("1".to_owned(), Some("".to_owned())));
        assert_eq!(
            Connection::agi_response_as_specialized_status::<Verbose>(response_body).unwrap(),
            AGIResponse::Ok(command::verbose::VerboseResponse {})
        );
    }

    #[test]
    fn parse_get_full_variable_incorrect() {
        let response_body = AGIMessage::Status(AGIStatusGeneric::Ok("2".to_owned(), None));
        assert!(
            Connection::agi_response_as_specialized_status::<GetFullVariable<ThisChannel>>(
                response_body
            )
            .is_err()
        );
    }

    #[test]
    fn set_variable_response_success() {
        let response_body = AGIMessage::Status(AGIStatusGeneric::Ok("0".to_owned(), None));
        assert!(
            Connection::agi_response_as_specialized_status::<SetVariable>(response_body).is_err()
        );
    }

    #[test]
    fn raw_command() {
        let response_body = AGIMessage::Status(AGIStatusGeneric::Ok("1".to_owned(), Some("stuff und so".to_owned())));
        assert_eq!(
            Connection::agi_response_as_specialized_status::<RawCommand>(response_body).unwrap(),
            AGIResponse::Ok(RawCommandResponse{ result: "1".to_owned(), op_data: Some("stuff und so".to_owned())})
        );
    }
}
