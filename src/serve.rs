//! Serve an existing [`Router`].
use std::sync::Arc;

use tokio::net::TcpListener;
#[cfg(feature = "tracing")]
extern crate tracing;
#[cfg(feature = "tracing")]
use tracing::{event, Level};

use crate::{router::Router, AGIError};

/// Actually serve a constructed Router, with a [`TcpListener`].
///
/// # Errors
/// Returns an Error when we are unable to start a [`TcpListener`].
pub async fn serve(listener: TcpListener, router: Router) -> Result<(), AGIError> {
    let router_arc = Arc::new(router);
    loop {
        let our_router = router_arc.clone();
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|_| AGIError::CannotSpawnListener)?;
        #[cfg(feature = "tracing")]
        event!(Level::DEBUG, "Got a new incoming connection.");
        tokio::spawn(async move {
            our_router.handle(stream).await;
        });
    }
}
