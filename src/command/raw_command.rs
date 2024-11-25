//! If you want to use a command that is not yet implemented, use the [`RawCommand`] defined here.
//! Please check if the command is already implemented and use the proper implementation if
//! available. In this way you can ensure much more type safety and have a nicer experience then
//! when using [`RawCommand`].
use super::*;

/// The RAW command.
///
/// Issue a raw command to asterisk - use this only if the command you need is not yet properly
/// implemented.
/// ```
/// use blazing_agi::command::RawCommand;
/// let cmd = RawCommand::new("GET DATA /some/file 10 7".to_owned());
/// // Will send (the \n simply terminates the command for asterisks parser):
/// assert_eq!(cmd.to_string(), "GET DATA /some/file 10 7\n")
/// ```
///
/// The associated [`InnerAGIResponse`] returned from [`send_command`](crate::connection::Connection::send_command) is
/// [`RawCommandResponse`].
#[derive(Debug)]
pub struct RawCommand {
    command: String,
}
impl RawCommand {
    /// Construct a RAW command. A trailing `\n` will be added, but no other interpolation will be
    /// made to this string before it is sent to asterisk.
    pub fn new(command: String) -> Self {
        Self { command }
    }
}
impl core::fmt::Display for RawCommand {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        writeln!(f, "{}", self.command)
    }
}
impl AGICommand for RawCommand {
    type Response = RawCommandResponse;
}

/// The responses we can get when sending [`RawCommand`] that returned 200.
/// No parsing happens on the return value - the response is simply destructured into the result
/// and operational data.
///
/// In other words, the literal string returned from asterisk is (written as a format string)
/// `200 result={result} {op_data.unwrap()}\n`
/// or
/// `200 result={result}\n` if `op_data` is None.
#[derive(Debug, PartialEq)]
pub struct RawCommandResponse {
    pub result: String,
    pub op_data: Option<String>,
}
impl InnerAGIResponse for RawCommandResponse {}
/// Convert from a tuple `(result, operational_data)` to [`RawCommandResponse`]. This is used
/// internally when parsing AGI responses to sending a [`RawCommand`] command.
impl<'a> TryFrom<(&'a str, Option<&'a str>)> for RawCommandResponse {
    type Error = AGIStatusParseError;
    fn try_from((result, op_data): (&str, Option<&str>)) -> Result<Self, Self::Error> {
        Ok(RawCommandResponse {
            result: result.to_owned(),
            op_data: op_data.map(|x| x.to_owned()),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn run_command() {
        let cmd = RawCommand::new("SAY DIGITS 1425 07".to_owned());
        assert_eq!(cmd.to_string(), "SAY DIGITS 1425 07\n");
    }

    #[test]
    fn parse_raw() {
        assert_eq!(
            RawCommandResponse::try_from(("0", Some("(stuff)"))).unwrap(),
            RawCommandResponse {
                result: "0".to_owned(),
                op_data: Some("(stuff)".to_owned())
            }
        );
    }
}
