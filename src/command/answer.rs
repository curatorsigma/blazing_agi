//! Defines the `Answer` AGI command
//! See also [the official
//! docs](https://docs.asterisk.org/Asterisk_22_Documentation/API_Documentation/AGI_Commands/answer/)
use super::*;

/// The Answer command.
///
/// Answer on the current channel.
/// Use with
/// ```
/// use blazing_agi::command::Answer;
/// let cmd = Answer::new();
/// // Will send:
/// assert_eq!(cmd.to_string(), "ANSWER\n")
/// ```
///
/// The associated [`InnerAGIResponse`] from [`send_command`](crate::connection::Connection::send_command) is
/// [`AnswerResponse`].
#[derive(Debug)]
pub struct Answer {}
impl Answer {
    /// Create the Answer command.
    pub fn new() -> Self {
        Self {}
    }
}

impl std::fmt::Display for Answer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ANSWER\n")
    }
}
impl AGICommand for Answer {
    type Response = AnswerResponse;
}

/// The responses we can get when sending [`Answer`] that returned 200.
#[derive(Debug, PartialEq)]
pub enum AnswerResponse {
    /// Successfully answer.
    Success,
    /// Failed to answer, but the problem was in Asterisk, not AGI.
    Failure,
}

impl InnerAGIResponse for AnswerResponse {}
/// Convert from a tuple `(result, operational_data)` to [`AnswerResponse`]. This is used
/// internally when parsing AGI responses to sending a [`Answer`] command.
impl<'a> TryFrom<(&'a str, Option<&'a str>)> for AnswerResponse {
    type Error = AGIStatusParseError;
    fn try_from((result, op_data): (&'a str, Option<&'a str>)) -> Result<Self, Self::Error> {
        let res_parsed = result.parse::<i32>();
        match res_parsed {
            Ok(0) => Ok(AnswerResponse::Success),
            Ok(-1) => Ok(AnswerResponse::Failure),
            _ => Err(AGIStatusParseError {
                result: result.to_owned(),
                op_data: op_data.map(|x| x.to_owned()),
                response_to_command: "ANSWER",
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn run() {
        let answer = Answer::new();
        assert_eq!(answer.to_string(), "ANSWER\n");
    }

    #[test]
    fn parse_success() {
        assert_eq!(
            AnswerResponse::try_from(("0", None)).unwrap(),
            AnswerResponse::Success
        );
    }

    #[test]
    fn parse_failure() {
        assert_eq!(
            AnswerResponse::try_from(("-1", Some("other stuff"))).unwrap(),
            AnswerResponse::Failure
        );
    }

    #[test]
    fn parse_incorrect_result() {
        assert_eq!(
            AnswerResponse::try_from(("1", None)),
            Err(AGIStatusParseError {
                result: "1".to_owned(),
                op_data: None,
                response_to_command: "ANSWER"
            })
        );
    }
}
