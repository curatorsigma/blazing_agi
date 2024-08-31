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
        write!(f, "VERBOSE \"{}\"\n", self.content)
    }
}
impl AGICommand for Verbose {
    type Response = VerboseResponse;
}

#[derive(Debug, PartialEq)]
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn run_empty_message() {
        let answer = Verbose::new("".to_string());
        assert_eq!(answer.to_string(), "VERBOSE \"\"\n");
    }

    #[test]
    fn run_non_empty_message() {
        let answer = Verbose::new("I am the debug output in asterisk".to_string());
        assert_eq!(answer.to_string(), "VERBOSE \"I am the debug output in asterisk\"\n");
    }

    #[test]
    fn parse_success() {
        assert_eq!(VerboseResponse::try_from(("1", None)).unwrap(), VerboseResponse {});
    }

    #[test]
    fn parse_incorrect_result() {
        assert_eq!(VerboseResponse::try_from(("0", Some("other stuff"))), Err(AGIStatusParseError{ result: "0".to_string(), op_data: Some("other stuff".to_string()), response_to_command: "VERBOSE"}) );
    }
}

