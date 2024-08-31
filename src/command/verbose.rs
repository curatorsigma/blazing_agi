use super::*;

#[derive(Debug)]
pub struct Verbose {
    content: String,
}
impl Verbose {
    pub fn new(s: String) -> Self {
        Self { content: s }
    }
}
impl std::fmt::Display for Verbose {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "VERBOSE \"{}\"", self.content)
    }
}
impl AGICommand for Verbose {
    type Response = VerboseResponse;
}

#[derive(Debug)]
pub struct VerboseResponse {}
impl InnerAGIResponse for VerboseResponse {
}
impl<'a> TryFrom<(&'a str, Option<&'a str>)> for VerboseResponse {
    type Error = AGIStatusParseError;
    fn try_from((result, op_data): (&str, Option<&str>)) -> Result<Self, Self::Error> {
        let res_parsed = result.parse::<u16>();
        match res_parsed {
            Ok(1) => {
                Ok(VerboseResponse {})
            }
            _ => {
                Err(AGIStatusParseError{ result: result.to_string() , op_data: op_data.map(|x| x.to_string()), response_to_command: "VERBOSE" })
            }
        }
    }
}

