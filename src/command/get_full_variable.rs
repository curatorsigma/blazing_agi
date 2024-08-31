use super::*;

pub trait ChannelSet: Send + Sync + std::fmt::Debug {}
#[derive(Debug)]
pub struct NotSet {}
impl ChannelSet for NotSet {}
#[derive(Debug)]
pub struct Set {
    channel_name: String,
}
impl ChannelSet for Set {}

#[derive(Debug)]
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
    pub fn on_channel(self, s: String) -> GetFullVariable<Set> {
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

#[derive(Debug)]
pub struct GetFullVariableResponse {
    pub value: Option<String>,
}
impl InnerAGIResponse for GetFullVariableResponse {
}
impl<'a> TryFrom<(&'a str, Option<&'a str>)> for GetFullVariableResponse {
    type Error = AGIStatusParseError;
    fn try_from((result, op_data): (&'a str, Option<&'a str>)) -> Result<Self, Self::Error> {
        let res_parsed = result.parse::<u16>();
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

