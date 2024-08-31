use super::*;

#[derive(Debug)]
pub struct Answer {
}
impl Answer {
    pub fn new() -> Self {
        Self { }
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

#[derive(Debug,PartialEq)]
pub enum AnswerResponse {
    Success,
    Failure,
}
impl InnerAGIResponse for AnswerResponse {
}
impl<'a> TryFrom<(&'a str, Option<&'a str>)> for AnswerResponse {
    type Error = AGIStatusParseError;
    fn try_from((result, op_data): (&'a str, Option<&'a str>)) -> Result<Self, Self::Error> {
        let res_parsed = result.parse::<i32>();
        match res_parsed {
            Ok(0) => {
                Ok(AnswerResponse::Success)
            },
            Ok(-1) => {
                Ok(AnswerResponse::Failure)
            }
            _ => {
                Err(AGIStatusParseError{ result: result.to_string(), op_data: op_data.map(|x| x.to_string()), response_to_command: "ANSWER" })
            },
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
        assert_eq!(AnswerResponse::try_from(("0", None)).unwrap(), AnswerResponse::Success);
    }

    #[test]
    fn parse_failure() {
        assert_eq!(AnswerResponse::try_from(("-1", Some("other stuff"))).unwrap(), AnswerResponse::Failure);
    }

    #[test]
    fn parse_incorrect_result() {
        assert_eq!(AnswerResponse::try_from(("1", None)), Err(AGIStatusParseError { result: "1".to_string(), op_data: None, response_to_command: "ANSWER"}));
    }
}

