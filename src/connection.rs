//! This module handles the literal network connection and sends/receives packets.
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{trace, Level};

use crate::*;

use self::agiparse::{AGIMessage, AGIParseError, AGIStatusGeneric};
use crate::command::{AGICommand, AGIResponse};

/// The buffers required while waiting for a full message to have arrived for parsing
#[derive(Debug)]
struct AGIMessageBuffer {
    /// The bytes read that belong to the next message we expect
    this_message: String,
}
impl AGIMessageBuffer {
    pub fn new() -> Self {
        AGIMessageBuffer {
            this_message: String::new(),
        }
    }

    /// Try to parse this_message as an AGIMessage
    pub fn try_parse_and_flush(&mut self) -> Result<Option<AGIMessage>, AGIParseError> {
        if self.this_message.len() == 0 {
            return Ok(None);
        };
        let msg = self.this_message.parse::<AGIMessage>()?;
        self.this_message = String::new();
        return Ok(Some(msg));
    }

    /// Given a single response from a tcp read, parse it and potentially return the next
    /// AGIMessage it contained
    ///
    /// The string passed here is assumed to contain no \0-bytes
    fn handle_single_call_buffer(
        &mut self,
        buf: &str,
    ) -> Result<Option<AGIMessage>, AGIParseError> {
        let mut last_newline_index = None;
        let mut current_line_start = 0;
        loop {
            current_line_start += last_newline_index.map_or(0_usize, |x| x + 1);
            last_newline_index = match buf[current_line_start..].find('\n') {
                // no more newline in message - copy it in its entirety to the buffer
                None => {
                    self.this_message.push_str(&buf[current_line_start..]);
                    // when the current message is a status, it is possible that the message is now
                    // complete and parsable. Try to parse it, but simply continue if that fails.
                    if line_type(&self.this_message) == LineType::Status {
                        let try_parse = self.try_parse_and_flush();
                        return match try_parse {
                            Ok(x) => Ok(x),
                            Err(_) => Ok(None),
                        };
                    } else {
                        return Ok(None);
                    }
                }
                // there was a newline. check what type the line is
                // (the newline IS PART OF the line, so we index ..= here)
                Some(x) => match line_type(&buf[current_line_start..=current_line_start + x]) {
                    LineType::Empty => {
                        let last_msg = self.try_parse_and_flush()?;
                        self.this_message.push_str(&buf[current_line_start + 1..]);
                        return Ok(last_msg);
                    }
                    LineType::Status => {
                        self.this_message.push_str(&buf[current_line_start..=current_line_start + x]);
                        let last_msg = self.try_parse_and_flush()?;
                        self.this_message.push_str(&buf[current_line_start + x + 1..]);
                        return Ok(last_msg);
                    }
                    LineType::NetworkStart => {
                        let last_msg = self.try_parse_and_flush()?;
                        if last_msg.is_some() {
                            return Err(AGIParseError::NetworkStartAfterOtherMessage);
                        };
                        self.this_message.push_str(&buf[current_line_start + x + 1..]);
                        return Ok(Some(AGIMessage::NetworkStart));
                    }
                    LineType::Unknown => {
                        self.this_message
                            .push_str(&buf[current_line_start..=current_line_start + x]);
                        Some(x)
                    }
                },
            };
        }
    }
}

/// `Connection` handles a single AGI stream (a connection originating from a client).
/// [`command`]s are sent with [`connection::Connection::send_command`](self::Connection::send_command)
#[derive(Debug)]
pub struct Connection {
    message_buf: AGIMessageBuffer,
    stream: TcpStream,
}
impl Connection {
    pub(crate) fn new(stream: TcpStream) -> Connection {
        Connection {
            message_buf: AGIMessageBuffer::new(),
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
    #[tracing::instrument(skip(self),level=Level::TRACE)]
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

    /// Read from TcpStream a single time and handle the result
    async fn read_single_call(&mut self) -> Result<Option<AGIMessage>, AGIParseError> {
        let mut ephemeral_buf = [0_u8; 2048];
        let bytes_read = self
            .stream
            .read(&mut ephemeral_buf)
            .await
            .map_err(|_| AGIParseError::ReadError)?;
        if bytes_read == 0 {
            return Err(AGIParseError::NoBytes);
        };
        let as_utf8 = ::std::str::from_utf8(&ephemeral_buf).map_err(|_| AGIParseError::NotUtf8)?;
        let first_zero_index = as_utf8.find('\0').unwrap_or(as_utf8.len());
        trace!("new bytes read from network in a single call: {as_utf8}");
        self.message_buf
            .handle_single_call_buffer(&as_utf8[0..first_zero_index])
    }

    /// Read the next message and parse it as an AGIMessage
    pub(crate) async fn read_and_parse(&mut self) -> Result<AGIMessage, AGIParseError> {
        // the message is potentially split across multiple TCP packets (or rather, TcpStream
        // `read`s.
        loop {
            let next_message_opt = self.read_single_call().await?;
            match next_message_opt {
                Some(x) => {
                    return Ok(x);
                }
                None => {}
            };
        }
    }
}

/// The type of a line in an agi message of unknown type
#[derive(Debug,PartialEq)]
enum LineType {
    /// agi_network: yes
    NetworkStart,
    /// no bytes in line
    Empty,
    /// status line of the format:
    /// \d\d\d result=.*
    Status,
    /// Anything else
    Unknown,
}
fn line_type(line: &str) -> LineType {
    if line == "\n" {
        LineType::Empty
    } else if line == "agi_network: yes\n" {
        LineType::NetworkStart
    } else if line.len() >= 3 && line[3..].starts_with(" result=") {
        LineType::Status
    } else {
        LineType::Unknown
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::command::{
        answer::{Answer, AnswerResponse},
        get_full_variable::{GetFullVariable, ThisChannel},
        raw_command::RawCommandResponse,
        verbose::Verbose,
        RawCommand, SetVariable,
    };

    use super::*;

    #[test]
    fn normal_network_start() {
        let mut message_buf = AGIMessageBuffer::new();
        let buf = "agi_network: yes\n";
        assert_eq!(
            message_buf.handle_single_call_buffer(buf),
            Ok(Some(AGIMessage::NetworkStart))
        );
        assert_eq!(message_buf.this_message, "".to_owned());
    }

    #[test]
    fn normal_vardump() {
        let mut message_buf = AGIMessageBuffer::new();
        let message = "\
            agi_network_script: agi.sh \n\
            agi_request: /tmp/agi.sh \n\
            agi_channel: SIP/marcelog-e00d2760 \n\
            agi_language: ar \n\
            agi_type: SIP \n\
            agi_uniqueid: 1297542965.8 \n\
            agi_version: 1.6.0.9 \n\
            agi_callerid: marcelog \n\
            agi_calleridname: marcelog@mg \n\
            agi_callingpres: 0 \n\
            agi_callingani2: 0 \n\
            agi_callington: 0 \n\
            agi_callingtns: 0 \n\
            agi_dnid: 667 \n\
            agi_rdnis: unknown \n\
            agi_context: default \n\
            agi_extension: 667 \n\
            agi_priority: 2 \n\
            agi_enhanced: 0.0 \n\
            agi_accountcode: \n\
            agi_threadid: 1104922960 \n\n";
        let vardump = message_buf
            .handle_single_call_buffer(message)
            .unwrap()
            .unwrap();
        assert_eq!(
            vardump,
            AGIMessage::VariableDump(AGIVariableDump {
                network_script: "agi.sh".to_owned(),
                request: agiparse::AGIRequestType::File(PathBuf::from("/tmp/agi.sh"),),
                channel: "SIP/marcelog-e00d2760".to_owned(),
                language: "ar".to_owned(),
                channel_type: "SIP".to_owned(),
                uniqueid: "1297542965.8".to_owned(),
                version: "1.6.0.9".to_owned(),
                callerid: "marcelog".to_owned(),
                calleridname: "marcelog@mg".to_owned(),
                callingpres: "0".to_owned(),
                callingani2: "0".to_owned(),
                callington: "0".to_owned(),
                callingtns: "0".to_owned(),
                dnid: "667".to_owned(),
                rdnis: "unknown".to_owned(),
                context: "default".to_owned(),
                extension: "667".to_owned(),
                priority: 2,
                enhanced: false,
                accountcode: "".to_owned(),
                threadid: 1104922960,
                custom_args: HashMap::<u8, String>::new(),
            })
        );
        assert_eq!(message_buf.this_message, "".to_owned());
    }

    #[test]
    fn normal_status() {
        let message = "200 result=1 done\n";
        let mut message_buf = AGIMessageBuffer::new();
        assert_eq!(
            message_buf.handle_single_call_buffer(message),
            Ok(Some(AGIMessage::Status(AGIStatusGeneric::Ok(
                "1".to_owned(),
                Some("done".to_owned())
            ))))
        );
    }


    #[test]
    fn status_split() {
        let message = "200 ";
        let mut message_buf = AGIMessageBuffer::new();
        assert_eq!(
            message_buf.handle_single_call_buffer(message),
            Ok(None));
        let msg2 = "result=1 done\n";
        assert_eq!(
            message_buf.handle_single_call_buffer(msg2),
            Ok(Some(AGIMessage::Status(AGIStatusGeneric::Ok(
                "1".to_owned(),
                Some("done".to_owned())
            ))))
        );
    }

    #[test]
    fn status_split_with_nonewline_packet() {
        let message = "200 ";
        let mut message_buf = AGIMessageBuffer::new();
        assert_eq!(
            message_buf.handle_single_call_buffer(message),
            Ok(None));
        let msg2 = "result";
        let nothing_yet = message_buf.handle_single_call_buffer(msg2);
        assert_eq!(nothing_yet, Ok(None));
        let msg3 = "=1 done\n";
        assert_eq!(
            message_buf.handle_single_call_buffer(msg3),
            Ok(Some(AGIMessage::Status(AGIStatusGeneric::Ok(
                "1".to_owned(),
                Some("done".to_owned())
            ))))
        );
    }

    #[test]
    fn netstart_plus_vardump_part() {
        let mut message_buf = AGIMessageBuffer::new();
        let msg1 = "agi_network: yes\n\
            agi_network_script: agi.sh \n\
            agi_request: /tmp/agi.sh \n\
            agi_channel: SIP/marcelog-e00d2760 \n\
            agi_language: ar \n\
            agi_type: SIP \n\
            agi_uniqueid: 1297542965.8 \n\
            agi_version: 1.6.0.9 \n\
            agi_callerid: marcelog \n";
        let msg1res = message_buf.handle_single_call_buffer(msg1);
        assert_eq!(msg1res, Ok(Some(AGIMessage::NetworkStart)));
        assert_eq!(message_buf.this_message, "agi_network_script: agi.sh \n\
            agi_request: /tmp/agi.sh \n\
            agi_channel: SIP/marcelog-e00d2760 \n\
            agi_language: ar \n\
            agi_type: SIP \n\
            agi_uniqueid: 1297542965.8 \n\
            agi_version: 1.6.0.9 \n\
            agi_callerid: marcelog \n");
        let msg2 = "\
            agi_calleridname: marcelog@mg \n\
            agi_callingpres: 0 \n\
            agi_callingani2: 0 \n\
            agi_callington: 0 \n\
            agi_callingtns: 0 \n\
            agi_dni";
        let nothing_yet = message_buf.handle_single_call_buffer(msg2);
        assert_eq!(nothing_yet, Ok(None));
        let msg3 = "\
            d: 667 \n\
            agi_rdnis: unknown \n\
            agi_context: default \n\
            agi_extension: 667 \n\
            agi_priority: 2 \n\
            agi_enhanced: 0.0 \n\
            agi_accountcode: \n\
            agi_threadid: 1104922960 \n\n";
        let vardump = message_buf
            .handle_single_call_buffer(msg3)
            .unwrap()
            .unwrap();
        assert_eq!(
            vardump,
            AGIMessage::VariableDump(AGIVariableDump {
                network_script: "agi.sh".to_owned(),
                request: agiparse::AGIRequestType::File(PathBuf::from("/tmp/agi.sh"),),
                channel: "SIP/marcelog-e00d2760".to_owned(),
                language: "ar".to_owned(),
                channel_type: "SIP".to_owned(),
                uniqueid: "1297542965.8".to_owned(),
                version: "1.6.0.9".to_owned(),
                callerid: "marcelog".to_owned(),
                calleridname: "marcelog@mg".to_owned(),
                callingpres: "0".to_owned(),
                callingani2: "0".to_owned(),
                callington: "0".to_owned(),
                callingtns: "0".to_owned(),
                dnid: "667".to_owned(),
                rdnis: "unknown".to_owned(),
                context: "default".to_owned(),
                extension: "667".to_owned(),
                priority: 2,
                enhanced: false,
                accountcode: "".to_owned(),
                threadid: 1104922960,
                custom_args: HashMap::<u8, String>::new(),
            })
        );
        assert_eq!(message_buf.this_message, "".to_owned());
    }

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
        let response_body = AGIMessage::Status(AGIStatusGeneric::Ok(
            "1".to_owned(),
            Some("stuff und so".to_owned()),
        ));
        assert_eq!(
            Connection::agi_response_as_specialized_status::<RawCommand>(response_body).unwrap(),
            AGIResponse::Ok(RawCommandResponse {
                result: "1".to_owned(),
                op_data: Some("stuff und so".to_owned())
            })
        );
    }
}
