use super::*;

pub trait ChannelSet: Send + Sync + std::fmt::Debug {}
#[derive(Debug,PartialEq)]
pub struct NotSet {}
impl ChannelSet for NotSet {}
#[derive(Debug,PartialEq)]
pub struct Set {
    channel_name: String,
}
impl ChannelSet for Set {}

#[derive(Debug,PartialEq)]
pub struct GetFullVariable<S: ChannelSet> {
    expression: String,
    channel_name: S,
}
impl GetFullVariable<NotSet> {
    pub fn new(s: String) -> Self {
        Self { expression: s, channel_name: NotSet {} }
    }
}
impl GetFullVariable<NotSet> {
    pub fn with_channel(self, s: String) -> GetFullVariable<Set> {
        GetFullVariable::<Set> { expression: self.expression, channel_name: Set{ channel_name: s }}
    }
}

impl std::fmt::Display for GetFullVariable<NotSet> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "GET FULL VARIABLE \"{}\"\n", self.expression)
    }
}
impl std::fmt::Display for GetFullVariable<Set> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "GET FULL VARIABLE \"{}\" \"{}\"\n", self.expression, self.channel_name.channel_name)
    }
}
impl AGICommand for GetFullVariable<Set> {
    type Response = GetFullVariableResponse;
}
impl AGICommand for GetFullVariable<NotSet> {
    type Response = GetFullVariableResponse;
}

#[derive(Debug,PartialEq)]
pub struct GetFullVariableResponse {
    pub value: Option<String>,
}
impl InnerAGIResponse for GetFullVariableResponse {
}
impl<'a> TryFrom<(&'a str, Option<&'a str>)> for GetFullVariableResponse {
    type Error = AGIStatusParseError;
    fn try_from((result, op_data): (&'a str, Option<&'a str>)) -> Result<Self, Self::Error> {
        let res_parsed = result.parse::<i32>();
        match res_parsed {
            Ok(1) => {
                match op_data {
                    Some(x) => {
                        let op_data_trimmed = x.trim_matches(|c| c == '(' || c == ')');
                        Ok(GetFullVariableResponse { value: Some(op_data_trimmed.to_string()) })
                    },
                    None => Err(AGIStatusParseError{ result: result.to_string(), op_data: None, response_to_command: "GET FULL VARIABLE" }),
                }
            },
            Ok(0) => {
                Ok(GetFullVariableResponse { value: None })
            }
            _ => {
                Err(AGIStatusParseError{ result: result.to_string(), op_data: op_data.map(|x| x.to_string()), response_to_command: "GET FULL VARIABLE" })
            },
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn run_empty_channel() {
        let answer = GetFullVariable::new("TEST_VAR_NAME".to_string());
        assert_eq!(answer.to_string(), "GET FULL VARIABLE \"TEST_VAR_NAME\"\n");
    }

    #[test]
    fn run_non_empty_channel() {
        let answer = GetFullVariable::new("TEST_VAR_NAME".to_string()).with_channel("The-Channel".to_string());
        assert_eq!(answer.to_string(), "GET FULL VARIABLE \"TEST_VAR_NAME\" \"The-Channel\"\n");
    }

    #[test]
    fn parse_success() {
        assert_eq!(GetFullVariableResponse::try_from(("1", Some("TheResult"))).unwrap(), GetFullVariableResponse { value: Some("TheResult".to_string()) });
    }

    #[test]
    fn parse_variable_does_not_exist() {
        assert_eq!(GetFullVariableResponse::try_from(("0", None)).unwrap(), GetFullVariableResponse { value: None });
    }

    #[test]
    fn parse_incorrect_result() {
        assert_eq!(GetFullVariableResponse::try_from(("-1", Some("irrelevant stuff"))), Err(AGIStatusParseError { result: "-1".to_string(), op_data: Some("irrelevant stuff".to_string()), response_to_command: "GET FULL VARIABLE"}));
    }
}

