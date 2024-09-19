//! Defines the `VERBOSE` AGI command.
//! See also the [official documentation](https://docs.asterisk.org/Asterisk_22_Documentation/API_Documentation/AGI_Commands/verbose/)
use super::*;

/// The Verbose command.
///
/// Send a message to asterisk debugging.
/// ```
/// use blazing_agi::command::Verbose;
/// let cmd = Verbose::new("Send this message".to_owned());
/// // Will send:
/// assert_eq!(cmd.to_string(), "VERBOSE \"Send this message\"\n")
/// ```
///
/// The associated [`InnerAGIResponse`] from [`send_command`](crate::connection::Connection::send_command) is
/// [`VerboseResponse`].
#[derive(Debug)]
pub struct Verbose {
    content: String,
}
impl Verbose {
    /// Construct a Verbose command. Will send `message` to asterisk when sent.
    pub fn new(message: String) -> Self {
        Self { content: message }
    }
}
impl std::fmt::Display for Verbose {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "VERBOSE \"{}\"", self.content)
    }
}
impl AGICommand for Verbose {
    type Response = VerboseResponse;
}

/// The responses we can get when sending [`Verbose`] that returned 200.
#[derive(Debug, PartialEq)]
pub struct VerboseResponse {}
impl InnerAGIResponse for VerboseResponse {}
/// Convert from a tuple `(result, operational_data)` to [`VerboseResponse`]. This is used
/// internally when parsing AGI responses to sending a [`Verbose`] command.
impl<'a> TryFrom<(&'a str, Option<&'a str>)> for VerboseResponse {
    type Error = AGIStatusParseError;
    fn try_from((result, op_data): (&str, Option<&str>)) -> Result<Self, Self::Error> {
        let res_parsed = result.parse::<u16>();
        match res_parsed {
            Ok(1) => Ok(VerboseResponse {}),
            _ => Err(AGIStatusParseError {
                result: result.to_owned(),
                op_data: op_data.map(|x| x.to_owned()),
                response_to_command: "VERBOSE",
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn run_empty_message() {
        let cmd = Verbose::new("".to_owned());
        assert_eq!(cmd.to_string(), "VERBOSE \"\"\n");
    }

    #[test]
    fn run_non_empty_message() {
        let cmd = Verbose::new("I am the debug output in asterisk".to_owned());
        assert_eq!(
            cmd.to_string(),
            "VERBOSE \"I am the debug output in asterisk\"\n"
        );
    }

    #[test]
    fn parse_success() {
        assert_eq!(
            VerboseResponse::try_from(("1", None)).unwrap(),
            VerboseResponse {}
        );
    }

    #[test]
    fn parse_incorrect_result() {
        assert_eq!(
            VerboseResponse::try_from(("0", Some("other stuff"))),
            Err(AGIStatusParseError {
                result: "0".to_owned(),
                op_data: Some("other stuff".to_owned()),
                response_to_command: "VERBOSE"
            })
        );
    }
}
