use std::sync::Arc;

use tokio::net::TcpListener;

use crate::{router::Router, AGIError};

/// Actually serve a constructed Router
pub async fn serve(listener: TcpListener, router: Router) -> Result<(), AGIError> {
    let router_arc = Arc::new(router);
    loop {
        let our_router = router_arc.clone();
        dbg!("waiting for next stream");
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|_| AGIError::CannotSpawnListener)?;
        dbg!("got new stream");
        tokio::spawn(async move {
            our_router.handle(stream).await;
        });
    }
}
