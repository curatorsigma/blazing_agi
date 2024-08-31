use blazing_agi::{
    command::RawCommand,
    router::Router,
    serve,
};
use blazing_agi_macros::create_handler;
use tokio::net::TcpListener;

#[create_handler]
async fn foo(connection: &mut Connection, request: &AGIRequest) -> Result<(), AGIError> {
    connection.send_command(RawCommand::new("SAY DIGITS 1234567 0".to_string())).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new().route("/script", foo);
    let listener = TcpListener::bind("0.0.0.0:4573").await?;
    serve::serve(listener, router).await?;
    Ok(())
}
