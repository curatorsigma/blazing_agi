use std::collections::HashMap;

use agiparse::{AGIMessage, AGIParseError, AGIStatus, AGIVariableDump};
use connection::Connection;
use handler::AGIHandler;

mod agiparse;
pub mod connection;
pub mod handler;
pub mod layer;
pub mod router;
pub mod serve;

#[derive(Debug)]
pub enum AGIError {
    NotFastAGI(AGIRequest),
    WrongSchema(String),
    NotEnoughCustomVariables(u8, u8),
    CannotSpawnListener,
    CannotSendCommand(tokio::io::Error),
    ParseError(AGIParseError),
    NotAStatus(AGIMessage),
}
impl std::fmt::Display for AGIError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NotFastAGI(_) => {
                write!(f, "The request is not a FastAGI request")
            }
            Self::WrongSchema(x) => {
                write!(f, "The schema {x} is not known")
            }
            Self::NotEnoughCustomVariables(x, y) => {
                write!(
                    f,
                    "There are only {x} custom variables, but {y} are required"
                )
            }
            Self::CannotSpawnListener => {
                write!(f, "Unable to spawn the TCP listener")
            }
            Self::CannotSendCommand(x) => {
                write!(f, "Unable to send an AGI command: {x}")
            }
            Self::ParseError(x) => {
                write!(f, "Unable to parse packet: {x}")
            }
            Self::NotAStatus(x) => {
                write!(f, "Sent a Command, but the response was not a Status: {x}")
            }
        }
    }
}
impl std::error::Error for AGIError {}


#[derive(Debug,PartialEq)]
pub enum AGICommand {
    Verbose(String),
    SetVariable(String, String),
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
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct AGIRequest {
    variables: AGIVariableDump,
    captures: HashMap<String, String>,
    wildcards: Option<String>,
}

#[cfg(test)]
mod tests {
    
}
