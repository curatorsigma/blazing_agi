# blazing_agi
`blazing_agi` is a fast, ergonomic and correct FastAGI Server, written in 100% safe Rust.

# Getting started
To get started, consider this "Hello World" example:
```rust
use blazing_agi::{command::{verbose::Verbose}, router::Router, serve};
use blazing_agi_macros::create_handler;
use tokio::net::TcpListener;

// The create_handler macro is used to turn an async fn into a handler.
// Make sure to use the same signature as here (including the variable names, but not the function
// name)
#[create_handler]
async fn foo(connection: &mut Connection, request: &AGIRequest) -> Result<(), AGIError> {
    connection.send_command(Verbose::new("Hello There".to_string())).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the router from the handlers you have defined
    let router = Router::new()
        .route("/script", foo);
    let listener = TcpListener::bind("0.0.0.0:4573").await?;
    // Start serving the Router
    serve::serve(listener, router).await?;
    Ok(())
}
```
You can find a more elaborate example in `examples/layer-agi-digest.rs`.
There, we use layering to add a Digest-Authentication Layer on top of a normal asterisk stream,
which requires minimal setup on the asterisk side, allowing secure authentication for endpoints
that should not be accessible by anyone.

In general, blazing_agi works by defining [`AGIHandler`] (read: scripts). You then combine them
into [`Router`](crate::router::Router)s. They define which requested uri is handled by which
handler.
An `AGIHandler` takes:
- a &mut `Connection` - this is a wrapper around a tokio `TcpStream`, which handles sending
Commands and parsing the response
- a & `AGIRequest` - this contains the data send in the initial request made by the client
(asterisk).

An `AGIHandler` can then use the `Connection::send_command` function to send commands to
the client.
When it is done, the Handler simply returns Ok(()) to signal that the
execution was successful and the stream can be terminated.
If an error is encountered that the Handler does not want to handle, it can be bubbled up as
`AGIError`, which tells the runtime that something went wrong - the stream is also closed.

# Limitations, Status and Stability
`blazing_agi` requires the use of tokio. Executor independence is currently not a goal.

`blazing_agi` does not currently contain definitions for all AGI commands.
Please file an issue or a PR if you want one added.

`blazing_agi` is currently pre-1.0. Consider pinning the exact version you use to ensure you get smooth `cargo update`s.

MSRV is `rustc 1.80`. The code may work on earlier versions, but I have not tested them.

# Contributing
I am very grateful for your help in improving `blazing_agi`!
If you need a feature, or have another suggestion for improving this project, please file an issue.
PRs are of course highly appreciated. As a rule:
- Use 100% safe Rust. (This is enforced by `#![forbid(unsafe_code)]`)
- Do not use `unwrap`. If a condition cannot fail, please use `except` *with a good explanation*.
- Ensure that `cargo test` passes.
- Use `cargo fmt` and consider using `cargo fix` before creating a PR.

## Open TODOs
### Implement a fallback command, that simply passes Strings to allow arbitrary commands
### Implement the remaining commands that asterisk allows.
You can find examples on the approach in `src/command/*.rs`.
Each command should get its own file in that directory and be re-exported by `crate::command`.
Each command should contain a way to construct itself (`new`, builderpattern where useful).
Each command must implement `crate::command::AGICommand`.
Here is a list of commands not currently implemented:
- ASYNC BREAK
- CHANNEL STATUS
- CONTROL STREAM FILE
- DATABASE DEL
- DATABASE DELTREE
- DATABASE GET
- DATABASE PUT
- EXEC
- GET DATA
- GET OPTION
- GET VARIABLE (NOTE: this may not be useful, since GET FULL VARIABLE is strictly more powerful and implemented)
- GOSUB
- HANGUP
- NOOP
- RECEIVE CHAR
- RECEIVE TEXT
- RECORD FILE
- SAY ALPHA
- SAY DATE
- SAY DATETIME
- SAY DIGITS
- SAY NUMBER
- SAY PHONETIC
- SAY TIME
- SEND IMAGE
- SEND TEXT
- SET AUTOHANGUP
- SET CALLERID
- SET CONTEXT
- SET EXTENSION
- SET MUSIC
- SET PRIORITY
- SPEECH ACTIVATE GRAMMAR
- SPEECH CREATE
- SPEECH DEACTIVATE GRAMMAR
- SPEECH DESTROY
- SPEECH LOAD GRAMMAR
- SPEECH RECOGNIZE
- SPEECH SET
- SPEECH UNLOAD GRAMMAR
- STREAM FILE
- TDD MODE
- WAIT FOR DIGIT

### Test as many commands against actual asterisk servers as possible.
I personally do not have use cases for most of the AGI commands, and not enough free time to dedicate to these integration tests.
If you use `blazing_agi` and find any bugs while integrating with asterisk, please file an issue.

## Have any question or comment?
Reach out to me, I would love to chat!


# License
This project is licensed under MIT-0 (MIT No Attribution).
By contributing to this repositry, you agree that your code will be licensed as MIT-0.

For my rationale for using MIT-0 instead of another more common license, please see
https://copy.church/objections/attribution/#why-not-require-attribution .

