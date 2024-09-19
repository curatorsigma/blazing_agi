//! Defines the `GET FULL VARIABLE` command and its possible responses.
//! See also [the official
//! docs](https://docs.asterisk.org/Asterisk_22_Documentation/API_Documentation/AGI_Commands/get_full_variable/)
use super::*;

// We encode the (potentially) set Channel as part of the Type
// Here we could have used an Option<String>, but in other cases we will need
// typestate patterns to ensure that only commands that are actuall allowed can even be built
pub trait TargetChannel: Send + Sync + std::fmt::Debug {}
#[derive(Debug, PartialEq)]
/// A variant of TargetChannel. Use the channel that originated the FastAGI call.
pub struct ThisChannel {}
impl TargetChannel for ThisChannel {}
#[derive(Debug, PartialEq)]
/// A variant of TargetChannel. Use the defined Channel.
pub struct OtherChannel {
    /// Use this channel name to evaluate the expression of the [`GetFullVariable`] command that
    /// uses this instance in its `TargetChannel`
    channel_name: String,
}
impl TargetChannel for OtherChannel {}

/// Implements the `GET FULL VARIABLE`  command in AGI
///
/// Evaluate an expression in a channel (defaults to own channel)
/// Build with
/// ```
/// use blazing_agi::command::GetFullVariable;
/// # use blazing_agi::command::get_full_variable::OtherChannel;
/// let cmd = GetFullVariable::new("TheExpression".to_owned())
///     // optional
///     .with_channel("TheChannel".to_owned());
/// // Will send:
/// assert_eq!(cmd.to_string(), "GET FULL VARIABLE \"TheExpression\" \"TheChannel\"\n")
/// ```
///
/// The associated [`InnerAGIResponse`] from [`send_command`](crate::connection::Connection::send_command) is
/// [`GetFullVariableResponse`].
#[derive(Debug, PartialEq)]
pub struct GetFullVariable<S: TargetChannel> {
    expression: String,
    channel_name: S,
}
/// With [`ThisChannel`] we signal that this command does not have a channel explicitly set.
/// You can use [`with_channel`](Self::with_channel) to set a channel.
impl GetFullVariable<ThisChannel> {
    /// Simple constructor, sets the expression to evaluate.
    pub fn new(expression: String) -> Self {
        Self {
            expression,
            channel_name: ThisChannel {},
        }
    }

    /// Set the channel on an instance that has no target channel set yet.
    pub fn with_channel(self, channel: String) -> GetFullVariable<OtherChannel> {
        GetFullVariable::<OtherChannel> {
            expression: self.expression,
            channel_name: OtherChannel {
                channel_name: channel,
            },
        }
    }
}

impl std::fmt::Display for GetFullVariable<ThisChannel> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "GET FULL VARIABLE \"{}\"", self.expression)
    }
}
impl std::fmt::Display for GetFullVariable<OtherChannel> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(
            f,
            "GET FULL VARIABLE \"{}\" \"{}\"",
            self.expression, self.channel_name.channel_name
        )
    }
}
impl AGICommand for GetFullVariable<OtherChannel> {
    type Response = GetFullVariableResponse;
}
impl AGICommand for GetFullVariable<ThisChannel> {
    type Response = GetFullVariableResponse;
}

/// The responses we can get after sending [`GetFullVariable`] that returns `200`.
#[derive(Debug, PartialEq)]
pub struct GetFullVariableResponse {
    /// `value` contains the value of the expression that was evaluated by asterisk.
    /// If the expression could not be evaluated (e.g. because a nonexistent function or variable was
    /// used), this will be `None`.
    pub value: Option<String>,
}
impl InnerAGIResponse for GetFullVariableResponse {}
/// Convert from a tuple `(result, operational_data)` to `GetFullVariableResponse`. This is used
/// internally when parsing AGI responses to sending a [`GetFullVariable`] command.
impl<'a> TryFrom<(&'a str, Option<&'a str>)> for GetFullVariableResponse {
    type Error = AGIStatusParseError;
    fn try_from((result, op_data): (&'a str, Option<&'a str>)) -> Result<Self, Self::Error> {
        let res_parsed = result.parse::<i32>();
        match res_parsed {
            Ok(1) => match op_data {
                Some(x) => {
                    let op_data_trimmed = x.trim_matches(|c| c == '(' || c == ')');
                    Ok(GetFullVariableResponse {
                        value: Some(op_data_trimmed.to_owned()),
                    })
                }
                None => Err(AGIStatusParseError {
                    result: result.to_owned(),
                    op_data: None,
                    response_to_command: "GET FULL VARIABLE",
                }),
            },
            Ok(0) => Ok(GetFullVariableResponse { value: None }),
            _ => Err(AGIStatusParseError {
                result: result.to_owned(),
                op_data: op_data.map(|x| x.to_owned()),
                response_to_command: "GET FULL VARIABLE",
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn run_empty_channel() {
        let cmd = GetFullVariable::new("TEST_VAR_NAME".to_owned());
        assert_eq!(cmd.to_string(), "GET FULL VARIABLE \"TEST_VAR_NAME\"\n");
    }

    #[test]
    fn run_non_empty_channel() {
        let cmd =
            GetFullVariable::new("TEST_VAR_NAME".to_owned()).with_channel("The-Channel".to_owned());
        assert_eq!(
            cmd.to_string(),
            "GET FULL VARIABLE \"TEST_VAR_NAME\" \"The-Channel\"\n"
        );
    }

    #[test]
    fn parse_success() {
        assert_eq!(
            GetFullVariableResponse::try_from(("1", Some("TheResult"))).unwrap(),
            GetFullVariableResponse {
                value: Some("TheResult".to_owned())
            }
        );
    }

    #[test]
    fn parse_variable_does_not_exist() {
        assert_eq!(
            GetFullVariableResponse::try_from(("0", None)).unwrap(),
            GetFullVariableResponse { value: None }
        );
    }

    #[test]
    fn parse_incorrect_result() {
        assert_eq!(
            GetFullVariableResponse::try_from(("-1", Some("irrelevant stuff"))),
            Err(AGIStatusParseError {
                result: "-1".to_owned(),
                op_data: Some("irrelevant stuff".to_owned()),
                response_to_command: "GET FULL VARIABLE"
            })
        );
    }
}
