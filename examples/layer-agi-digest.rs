use std::fmt::Display;

use async_trait::async_trait;
use blazing_agi::{
    command::{get_full_variable::GetFullVariable, verbose::Verbose, AGIResponse},
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

/// Create a 20-byte Nonce with 8 bytes of Randomness, encoded as a hex string
fn create_nonce() -> String {
    let mut raw_bytes = [0_u8; 20];
    let now_in_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Should be after the epoch");
    // 8 bytes against reuse
    raw_bytes[0..=7].clone_from_slice(&now_in_secs.as_secs().to_le_bytes());
    // 4 bytes against reuse
    raw_bytes[8..=11].clone_from_slice(&now_in_secs.subsec_millis().to_le_bytes());
    // 8 bytes against predictability
    rand::rngs::ThreadRng::default().fill(&mut raw_bytes[12..=19]);
    return hex::encode(raw_bytes);
}

#[derive(Clone, Debug)]
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
    //
    // In asterisk, you have to set the same secret as follows:
    // same => n,Set(BLAZING_AGI_DIGEST_SECRET=top_secret)
    async fn handle(&self, connection: &mut Connection, _: &AGIRequest) -> Result<(), AGIError> {
        let nonce = create_nonce();
        let mut hasher = Sha1::new();
        hasher.update(self.secret.as_bytes());
        hasher.update(":".as_bytes());
        hasher.update(&nonce.as_bytes());
        let expected_digest: [u8; 20] = hasher.finalize().into();
        let digest_response = connection
            .send_command(GetFullVariable::new(format!(
                "${{SHA1(${{BLAZING_AGI_DIGEST_SECRET}}:{})}}",
                nonce
            )))
            .await?;
        match digest_response {
            AGIResponse::Ok(inner_response) => {
                if let Some(digest_as_str) = inner_response.value {
                    if expected_digest
                        != *hex::decode(digest_as_str).map_err(|_| {
                            AGIError::InnerError(Box::new(SHA1DigestError::DecodeError))
                        })?
                    {
                        connection
                            .send_command(Verbose::new(
                                "Unauthenticated: Wrong Digest.".to_string(),
                            ))
                            .await?;
                        Err(AGIError::InnerError(Box::new(SHA1DigestError::WrongDigest)))
                    } else {
                        Ok(())
                    }
                } else {
                    Err(AGIError::ClientSideError(
                        "Expected BLAZING_AGI_DIGEST_SECRET to be set, but it is not".to_string(),
                    ))
                }
            }
            m => {
                return Err(AGIError::Not200(m.into()));
            }
        }
    }
}

#[create_handler]
async fn foo(connection: &mut Connection, request: &AGIRequest) -> Result<(), AGIError> {
    connection
        .send_command(Verbose::new("Hello There!".to_string()))
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new().route(
        "/protected/foo",
        // The and_then macro takes two handlers and combines them:
        // The first will run; then the second will run if the first returned Ok(())
        and_then!((SHA1DigestOverAGI::new("top_secret"), foo)),
    );
    // But this is even nicer if you use a layer:
    // Here, every route added !before! the layer will have the digest running first.
    let _router_equivalent = Router::new()
        .route("/protected/foo", foo)
        .layer(layer_before!(SHA1DigestOverAGI::new("top_secret")))
        // this route will NOT have the SHA1 Digest running
        .route("/not_protected/foo", foo);

    let listener = TcpListener::bind("0.0.0.0:4573").await?;
    serve(listener, router).await?;
    Ok(())
}
