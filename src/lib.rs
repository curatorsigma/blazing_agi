//! blazing_agi is a fast, ergonomic and correct FastAGI Server.
//!
//! blazing_agi requires the use of tokio. Executor independence is currently not a goal.
//! blazing_agi does not currently contain definitions for all AGI commands. Please file an issue
//! or a PR if you want one added.
//!
//! To get started, consider this "Hello World" example:
//! ```ignore
//! use blazing_agi::{command::{verbose::Verbose}, router::Router, serve};
//! use blazing_agi_macros::create_handler;
//! use tokio::net::TcpListener;
//!
//! // The create_handler macro is used to turn an async fn into a handler.
//! // Make sure to use the same signature as here (including the variable names, but not the function
//! // name)
//! #[create_handler]
//! async fn foo(connection: &mut Connection, request: &AGIRequest) -> Result<(), AGIError> {
//!     connection.send_command(Verbose::new("Hello There".to_string())).await?;
//!     Ok(())
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create the router from the handlers you have defined
//!     let router = Router::new()
//!         .route("/script", foo);
//!     let listener = TcpListener::bind("0.0.0.0:5473").await?;
//!     // Start serving the Router
//!     serve::serve(listener, router).await?;
//!     Ok(())
//! }
//! ```
//! You can find a more elaborate example in `examples/layer-agi-digest.rs`.
//!
//! In general, blazing_agi works by defining [`AGIHandler`] (read: scripts). You then combine them
//! into [`Router`](crate::router::Router)s. They define which requested uri is handled by which
//! handler.
//! An [`AGIHandler`] takes:
//! - a &mut [`Connection`] - this is a wrapper around a tokio `TcpStream`, which handles sending
//! Commands and parsing the response
//! - a & [`AGIRequest`] - this contains the data send in the initial request made by the client
//! (asterisk).
//!
//! An [`AGIHandler`] can then use the [`Connection::send_command`] function to send commands to
//! the client.
//! When it is done, the Handler simply returns Ok(()) to signal that the
//! execution was successful and the stream can be terminated.
//! If an error is encountered that the Handler does not want to handle, it can be bubbled up as
//! [`AGIError`], which tells the runtime that something went wrong - the stream is also closed.
use std::collections::HashMap;

use agiparse::{AGIMessage, AGIParseError, AGIStatusGeneric, AGIVariableDump};
use connection::Connection;
use handler::AGIHandler;

mod agiparse;
pub mod command;
pub mod connection;
pub mod handler;
pub mod layer;
pub mod router;
pub mod serve;

/// Contains all the ways in which serving a FastAGI Request can fail.
#[derive(Debug)]
pub enum AGIError {
    /// Handlers may use this to bubble up errors if they want.
    InnerError(Box<dyn std::error::Error>),
    /// A special case:
    /// This is raised when the client (asterisk) made a well-formed request
    /// with incorrect data (such as Unauth etc) - the handler asks the router to break
    /// communication with the client.
    ClientSideError(String),
    /// Expected a 200-response, but got something else.
    Not200(u16),
    /// The request was a normal AGI request over the network.
    NotFastAGI(AGIRequest),
    /// agi:// was not chosen as the schema.
    WrongSchema(String),
    /// A handler expected (param 1) custom arguments, but only (param 2) were actually passed.
    NotEnoughCustomVariables(u8, u8),
    /// Unable to spawn a TcpListener.
    CannotSpawnListener,
    /// Unable to send a command.
    CannotSendCommand(tokio::io::Error),
    /// Unable to parse an incoming packet.
    ParseError(AGIParseError),
    /// A parsable message came in. We expected a Status, but got something else.
    NotAStatus(AGIMessage),
    /// The generic AGI status could be read, the expected return type is known, but the response
    /// actually received is not parsable as the special response type expected.
    AGIStatusUnspecializable(AGIStatusGeneric, &'static str),
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
            Self::ClientSideError(x) => {
                write!(f, "Error on the Client side: {x}")
            }
            Self::AGIStatusUnspecializable(x, y) => {
                write!(f, "I am unable to specialize {x} as a response to {y}")
            }
        }
    }
}
impl std::error::Error for AGIError {}

/// The Data sent with the request.
#[derive(Debug, PartialEq)]
pub struct AGIRequest {
    /// The individual variables that asterisk sent.
    pub variables: AGIVariableDump,
    /// The pathsegments of the request uri that were captured in :capture segments.
    pub captures: HashMap<String, String>,
    /// The pathsegments of the request uri that were captured in * segments.
    pub wildcards: Option<String>,
}
