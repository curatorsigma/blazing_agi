// Reexport all the files in src/command/
// They should contain one type of command each
pub mod answer;
pub use self::answer::Answer as Answer;
pub mod verbose;
pub use self::verbose::Verbose as Verbose;
pub mod get_full_variable;
pub use self::get_full_variable::GetFullVariable as GetFullVariable;
pub mod set_variable;
pub use self::set_variable::SetVariable as SetVariable;

/// An Error that occured while converting an AGIStatusGeneric to a specialized response
#[derive(Debug,PartialEq)]
pub struct AGIStatusParseError {
    result: String,
    op_data: Option<String>,
    pub(crate) response_to_command: &'static str,
}
impl std::fmt::Display for AGIStatusParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Unable to parse result {}, data {:?} as {} Status.", self.result, self.op_data, self.response_to_command)
    }
}
impl std::error::Error for AGIStatusParseError {}

/// These are the different responses we can get to our AGI commands.
/// In this Enum, the response is fully parsed and will look different for each command in the Ok
/// case
#[derive(Debug,PartialEq)]
pub enum AGIResponse<H> where H: InnerAGIResponse + Sized {
    /// 200 - The Inner Value is the fully parsed Response and depends on the command
    /// sent.
    Ok(H),
    /// 510 - Asterisk thinks, this command is invalid
    Invalid,
    /// 511 - The Channel no longer exists
    DeadChannel,
    /// 520 - TODO
    EndUsage,
}
/// Convert a Response back into its response code
impl<H> Into<u16> for AGIResponse<H> where H: InnerAGIResponse + Sized {
    fn into(self) -> u16 {
        match self {
            AGIResponse::Ok(_) => 200,
            AGIResponse::Invalid => 510,
            AGIResponse::DeadChannel => 511,
            AGIResponse::EndUsage => 520,
        }
    }
}

/// The part of the 200(Ok)-Case response that is specific to the issued Command
/// See examples in src/command/*.rs
pub trait InnerAGIResponse: std::fmt::Debug + for<'a> TryFrom<(&'a str, Option<&'a str>), Error = AGIStatusParseError>  + Send + Sync {}

/// A command that can be issued via AGI. See examples in src/command/*.rs
pub trait AGICommand: std::fmt::Display + std::fmt::Debug + Send + Sync {
    type Response: InnerAGIResponse;
}


/// Characters a user can type when getting DTMF data
#[derive(Debug,PartialEq)]
pub enum Characters {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Star,
    Pound,
}

/// Digits a user can type
#[derive(Debug,PartialEq)]
pub enum Digit {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
}
impl Into<Characters> for Digit{
    fn into(self) -> Characters {
        match self {
            Digit::Zero => Characters::Zero,
            Digit::One => Characters::One,
            Digit::Two => Characters::Two,
            Digit::Three => Characters::Three,
            Digit::Four => Characters::Four,
            Digit::Five => Characters::Five,
            Digit::Six => Characters::Six,
            Digit::Seven => Characters::Seven,
            Digit::Eight => Characters::Eight,
            Digit::Nine => Characters::Nine,
        }
    }
}

