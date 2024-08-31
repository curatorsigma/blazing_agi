use blazing_agi::{
    command::verbose::Verbose,
    router::Router,
    serve,
};
use blazing_agi_macros::create_handler;
use tokio::net::TcpListener;

// The create_handler macro is used to turn an async fn into a handler.
// Make sure to use the same signature as here (including the variable names, but not the function
// name)
#[create_handler]
async fn foo(connection: &mut Connection, request: &AGIRequest) -> Result<(), AGIError> {
    Ok(())
}

#[create_handler]
async fn foo2(connection: &mut Connection, request: &AGIRequest) -> Result<(), AGIError> {
    connection
        .send_command(Verbose::new("hi there".to_string()))
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the router from the handlers you have defined
    let router = Router::new().route("/script", foo).route("/other", foo2);
    let listener = TcpListener::bind("0.0.0.0:5473").await?;
    // Start serving the Router.
    serve::serve(listener, router).await?;
    Ok(())
}
