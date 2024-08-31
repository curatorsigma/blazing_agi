//! Defines the `SET VARIABLE` AGI command.
//! See also the [official documentation](https://docs.asterisk.org/Asterisk_22_Documentation/API_Documentation/AGI_Commands/get_full_variable/)
use super::*;

/// The Set Variable command.
///
/// Use with
/// ```
/// use blazing_agi::command::SetVariable;
/// let cmd = SetVariable::new("TheVariable".to_string(), "TheValue".to_string());
/// // Will send:
/// assert_eq!(cmd.to_string(), "SET VARIABLE \"TheVariable\" \"TheValue\"\n")
/// ```
///
/// The associated [`InnerAGIResponse`] from [`send_command`](crate::connection::Connection::send_command) is
/// [`SetVariableResponse`].
#[derive(Debug)]
pub struct SetVariable {
    var_name: String,
    value: String,
}
impl SetVariable {
    /// Create [`SetVariable`]. When sent, this will set `var_name` to `value`.
    pub fn new(var_name: String, value: String) -> Self {
        Self { var_name, value }
    }
}
impl std::fmt::Display for SetVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "SET VARIABLE \"{}\" \"{}\"\n", self.var_name, self.value)
    }
}
impl AGICommand for SetVariable {
    type Response = SetVariableResponse;
}

/// The responses we can get when sending [`SetVariableResponse`] that returned 200.
/// There is only one acceptable response: `200 result=1`, so this is the empty struct.
#[derive(Debug, PartialEq)]
pub struct SetVariableResponse {}
impl InnerAGIResponse for SetVariableResponse {}
/// Convert from a tuple `(result, operational_data)` to [`SetVariableResponse`]. This is used
/// internally when parsing AGI responses to sending a [`SetVariable`] command.
impl<'a> TryFrom<(&'a str, Option<&'a str>)> for SetVariableResponse {
    type Error = AGIStatusParseError;
    fn try_from((result, op_data): (&str, Option<&str>)) -> Result<Self, Self::Error> {
        let res_parsed = result.parse::<u16>();
        match res_parsed {
            Ok(1) => {
                Ok(SetVariableResponse {})
            }
            _ => {
                Err(AGIStatusParseError{ result: result.to_string() , op_data: op_data.map(|x| x.to_string()), response_to_command: "SET VARIABLE" })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn run_normal_set() {
        let cmd = SetVariable::new("TEST_VAR_NAME".to_string(), "the-value".to_string());
        assert_eq!(cmd.to_string(), "SET VARIABLE \"TEST_VAR_NAME\" \"the-value\"\n");
    }

    #[test]
    fn parse_success() {
        assert_eq!(SetVariableResponse::try_from(("1", None)).unwrap(), SetVariableResponse {});
    }

    #[test]
    fn parse_incorrect_result() {
        assert_eq!(SetVariableResponse::try_from(("0", Some("other stuff"))), Err(AGIStatusParseError{ result: "0".to_string(), op_data: Some("other stuff".to_string()), response_to_command: "SET VARIABLE"}) );
    }
}

