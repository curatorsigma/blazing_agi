use blazing_agi::{command::AGICommand, router::Router, serve};
use blazing_agi_macros::create_handler;
use tokio::net::TcpListener;

#[create_handler]
async fn foo(connection: &mut Connection, request: &AGIRequest) -> Result<(), AGIError> {
    Ok(())
}

#[create_handler]
async fn foo2(connection: &mut Connection, request: &AGIRequest) -> Result<(), AGIError> {
    connection
        .send_command(AGICommand::Verbose("hi there".to_string()))
        .await
        .unwrap();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new().route("/script", foo).route("/other", foo2);
    let listener = TcpListener::bind("0.0.0.0:5473").await?;
    serve::serve(listener, router).await?;
    Ok(())
}
