use std::collections::HashMap;

use agiparse::{AGIMessage, AGIParseError, AGIStatus, AGIVariableDump};
use connection::Connection;
use handler::AGIHandler;

mod agiparse;
pub mod command;
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
    InnerError(Box<dyn std::error::Error>),
    Not200(u16),
    NoOperationalData(AGIStatus),
    /// A special case:
    /// This is raised when the client (asterisk) made a well-formed request
    /// with incorrect data (such as Unauth etc) - the handler asks the router to break
    /// communication with the client
    ClientSideError(String),
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
            Self::InnerError(x) => {
                write!(f, "InnerError: {x}")
            }
            Self::Not200(x) => {
                write!(f, "Handler expected 200-response, but got {x}")
            }
            Self::NoOperationalData(x) => {
                write!(
                    f,
                    "Handler expected status with operational data, but got {x}"
                )
            }
            Self::ClientSideError(x) => {
                write!(f, "Error on the Client side: {x}")
            }
        }
    }
}
impl std::error::Error for AGIError {}

#[derive(Debug, PartialEq)]
pub struct AGIRequest {
    pub variables: AGIVariableDump,
    pub captures: HashMap<String, String>,
    pub wildcards: Option<String>,
}

#[cfg(test)]
mod tests {}
