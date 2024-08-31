//! This module contains all AGI commands that can be sent.
//!
//! If you want to send an AGI command, you will need to:
//! ```ignore
//! // import some things
//! use blazing_agi::command::AGIResponse;
//! use blazing_agi::AGIError;
//! use blazing_agi::command::SetVariable;
//!
//! # async fn main() -> Result<(), AGIError> {
//! // Create the appropriate Command
//! let cmd = SetVariable::new("VarName".to_string(), "Value".to_string());
//! // and send it over a connection
//! let conn: blazing_agi::connection::Connection = todo!();
//! let res = conn.send_command(cmd).await;
//! // This may fail for various reasons and you may want to destructure it:
//! let response = match res {
//!     Err(e) => { todo!(); }
//!     Ok(x) => { x }
//! };
//! // You get an `AGIResponse`
//! match response {
//!     // asterisk returned 200
//!     AGIResponse::Ok(inner_result) => {
//!         // inner_result has a type that depends on the command you sent.
//!         // You will find the documentation for the command linking to it.
//!         // For SetVariable, the response is SetVariableResponse, which is the unit struct
//!         println!("Success :)");
//!     }
//!     AGIResponse::Invalid => {
//!         // You can early return errors in handlers with this pattern:
//!         return Err(AGIError::Not200(response.into()));
//!     }
//!     _ => {
//!         println!("There are other variants for the 5xx codes.");
//!     }
//! };
//! # }
//! ```

// Reexport all the files in src/command/
// They should contain one type of command each
pub mod raw_command;
pub use self::raw_command::RawCommand;

pub mod answer;
pub use self::answer::Answer;
pub mod verbose;
pub use self::verbose::Verbose;
pub mod get_full_variable;
pub use self::get_full_variable::GetFullVariable;
pub mod set_variable;
pub use self::set_variable::SetVariable;

/// An Error that occured while converting an AGIStatusGeneric to a specialized response.
#[derive(Debug, PartialEq)]
pub struct AGIStatusParseError {
    result: String,
    op_data: Option<String>,
    pub(crate) response_to_command: &'static str,
}
impl std::fmt::Display for AGIStatusParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Unable to parse result {}, data {:?} as {} Status.",
            self.result, self.op_data, self.response_to_command
        )
    }
}
impl std::error::Error for AGIStatusParseError {}

/// These are the different responses we can get to our AGI commands.
/// In this Enum, the response is fully parsed and will look different for each command in the Ok
/// case.
#[derive(Debug, PartialEq)]
pub enum AGIResponse<H>
where
    H: InnerAGIResponse + Sized,
{
    /// 200 - The Inner Value is the fully parsed Response and depends on the command
    /// sent.
    Ok(H),
    /// 510 - Asterisk thinks, this command is invalid.
    Invalid,
    /// 511 - The Channel no longer exists.
    DeadChannel,
    /// 520 - TODO
    EndUsage,
}
/// Convert a Response back into its response code
impl<H> Into<u16> for AGIResponse<H>
where
    H: InnerAGIResponse + Sized,
{
    fn into(self) -> u16 {
        match self {
            AGIResponse::Ok(_) => 200,
            AGIResponse::Invalid => 510,
            AGIResponse::DeadChannel => 511,
            AGIResponse::EndUsage => 520,
        }
    }
}

/// The part of the 200(Ok)-Case response that is specific to the issued Command.
/// The appropriate Response type will be listed under each Command in [`crate::command`].
pub trait InnerAGIResponse:
    std::fmt::Debug
    + for<'a> TryFrom<(&'a str, Option<&'a str>), Error = AGIStatusParseError>
    + Send
    + Sync
{
}

/// A command that can be issued via AGI. See examples in src/command/*.rs
pub trait AGICommand: std::fmt::Display + std::fmt::Debug + Send + Sync {
    type Response: InnerAGIResponse;
}

/// Characters a user can type when getting DTMF data
#[derive(Debug, PartialEq)]
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
#[derive(Debug, PartialEq)]
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
impl Into<Characters> for Digit {
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
