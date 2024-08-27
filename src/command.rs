/// AGI Commands that we can issue
#[derive(Debug, PartialEq)]
pub enum AGICommand {
    Verbose(String),
    SetVariable(String, String),
    /// Sends the following line:
    /// `GET FULL VARIABLE x [y]`
    /// This interpolates the first value in the channel given by the second value
    /// or the current channel if the second value is None
    GetFullVariable(String, Option<String>),
}
impl std::fmt::Display for AGICommand {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Verbose(msg) => {
                write!(f, "VERBOSE \"{msg}\"")
            }
            Self::SetVariable(name, value) => {
                write!(f, "SET VARIABLE \"{name}\" \"{value}\"")
            }
            Self::GetFullVariable(expr, channel) => {
                if let Some(x) = channel {
                    write!(f, "GET FULL VARIABLE \"{expr}\" \"{x}\"")
                } else {
                    write!(f, "GET FULL VARIABLE \"{expr}\"")
                }
            }
        }
    }
}
