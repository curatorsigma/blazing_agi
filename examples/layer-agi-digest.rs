use std::fmt::Display;

use async_trait::async_trait;
use blazing_agi::{
    command::AGICommand,
    connection::Connection,
    handler::{AGIHandler, AndThenHandler},
    router::Router,
    serve::serve,
    AGIError, AGIRequest,
};
use blazing_agi_macros::{and_then, create_handler, layer_before};
use rand::Rng;
use sha1::{Digest, Sha1};
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
enum SHA1DigestError {
    DecodeError,
    WrongDigest,
}
impl Display for SHA1DigestError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::DecodeError => {
                write!(f, "The returned digest was not decodable as u8")
            }
            Self::WrongDigest => {
                write!(f, "The returned digest is false")
            }
        }
    }
}
impl std::error::Error for SHA1DigestError {}

fn create_nonce() -> String {
    let mut raw_bytes: Vec<u8> = vec![];
    let now_in_secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("Should be after the epoch");
    for b in now_in_secs.as_secs().to_le_bytes() {
        raw_bytes.push(b);
    }
    for b in now_in_secs.subsec_millis().to_le_bytes() {
        raw_bytes.push(b);
    }
    let mut buf = [0_u8; 8];
    rand::rngs::ThreadRng::default().fill(&mut buf);
    for b in buf.iter() {
        raw_bytes.push(*b);
    }
    return hex::encode(raw_bytes);
}

#[derive(Clone)]
struct SHA1DigestOverAGI {
    secret: String,
}
impl SHA1DigestOverAGI {
    pub fn new<S: AsRef<str>>(secret: S) -> Self {
        Self {
            secret: secret.as_ref().to_string(),
        }
    }
}
#[async_trait]
impl AGIHandler for SHA1DigestOverAGI {
    // Note: this handler does not care about the request.
    // It simply ignores it and does the AGI digest.
    // This handler effectively works as a layer later)
    async fn handle(&self, connection: &mut Connection, _: &AGIRequest) -> Result<(), AGIError> {
        let nonce = create_nonce();
        let mut hasher = Sha1::new();
        hasher.update(self.secret.as_bytes());
        hasher.update(":".as_bytes());
        hasher.update(&nonce.as_bytes());
        let expected_digest: [u8; 20] = hasher.finalize().into();
        let digest_response = connection
            .send_command(AGICommand::GetFullVariable(
                format!("${{SHA1(${{BLAZING_AGI_DIGEST_SECRET}}:{})}}", nonce),
                None,
            ))
            .await?;
        if digest_response.code != 200 {
            return Err(AGIError::Not200(digest_response.code));
        };
        if let Some(x) = digest_response.operational_data {
            let digest_as_str = x.trim_matches(|c| c == '(' || c == ')');
            if expected_digest != *hex::decode(digest_as_str)
                    .map_err(|_| AGIError::InnerError(Box::new(SHA1DigestError::DecodeError)))? {
                connection.send_command(AGICommand::Verbose("Unauthenticated: Wrong Digest.".to_string())).await?;
                Err(AGIError::InnerError(Box::new(SHA1DigestError::WrongDigest)))
            } else {
                Ok(())
            }
        } else {
            Err(AGIError::NoOperationalData(digest_response))
        }
    }
}

#[create_handler]
async fn foo(connection: &mut Connection, request: &AGIRequest) -> Result<(), AGIError> {
    connection
        .send_command(AGICommand::Verbose("Hello There!".to_string()))
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new().route(
        "/protected/foo",
        and_then!((SHA1DigestOverAGI::new("top_secret"), foo)),
    );
    // But this is even nicer, if you use a layer:
    // Here, every route added !before! the layer will have the digest running first.
    let _router_equivalent = Router::new()
        .route("/protected/foo", foo)
        .layer(layer_before!(SHA1DigestOverAGI::new("top_secret")));

    let listener = TcpListener::bind("172.21.0.31:4573").await?;
    serve(listener, router).await?;
    Ok(())
}
