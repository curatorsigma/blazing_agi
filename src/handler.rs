//! Defines the [`AGIHandler`], the most basic instrument for answering FastAGI requests.
use tracing::Level;

use crate::{command::verbose::Verbose, AGIError, AGIRequest, Connection};

/// The main trait that handles an AGI request.
///
/// Using this crate usually boils down to creating a [`Router`](crate::router::Router) from [`AGIHandler`](self::AGIHandler)s.
/// If the Handler needs no state, consider using the [`create_handler!`](`blazing_agi_macros::create_handler`) macro
/// for converting async fn into AGIHandler.
/// If your handler needs state between different requests, you may want to manually impl
/// AGIHandler. Make sure to use `#[async_trait::async_trait]` for your impl block.
#[async_trait::async_trait]
pub trait AGIHandler: Send + Sync + std::fmt::Debug {
    async fn handle(
        &self,
        connection: &mut Connection,
        request: &AGIRequest,
    ) -> Result<(), AGIError>;
}

#[async_trait::async_trait]
impl AGIHandler for Box<dyn AGIHandler> {
    async fn handle(&self, conn: &mut Connection, req: &AGIRequest) -> Result<(), AGIError> {
        (**self).handle(conn, req).await
    }
}

#[async_trait::async_trait]
impl AGIHandler for &Box<dyn AGIHandler> {
    async fn handle(&self, conn: &mut Connection, req: &AGIRequest) -> Result<(), AGIError> {
        (**self).handle(conn, req).await
    }
}

#[async_trait::async_trait]
impl AGIHandler for &dyn AGIHandler {
    async fn handle(&self, conn: &mut Connection, req: &AGIRequest) -> Result<(), AGIError> {
        (**self).handle(conn, req).await
    }
}

/// Apply one handler, and if that succeeded another afterwards.
/// You can build this handler with [`and_then!`](blazing_agi_macros::and_then).
#[derive(Debug)]
pub struct AndThenHandler {
    first: Box<dyn AGIHandler>,
    second: Box<dyn AGIHandler>,
}
impl AndThenHandler {
    /// Given the two handlers, the first one will be executed. If it succeeded, the second one
    /// will also be executed.
    pub fn new(first: Box<dyn AGIHandler>, second: Box<dyn AGIHandler>) -> Self {
        AndThenHandler { first, second }
    }
}
#[async_trait::async_trait]
impl AGIHandler for AndThenHandler {
    async fn handle(
        &self,
        connection: &mut Connection,
        request: &AGIRequest,
    ) -> Result<(), AGIError> {
        self.first.handle(connection, request).await?;
        self.second.handle(connection, request).await
    }
}

/// A trivial AGI response, simply acknowledging that a route does not exist.
#[derive(Debug)]
pub(crate) struct FallbackHandler {}
#[async_trait::async_trait]
impl AGIHandler for FallbackHandler {
    #[tracing::instrument(level=Level::DEBUG, ret, err)]
    async fn handle(&self, connection: &mut Connection, _: &AGIRequest) -> Result<(), AGIError> {
        connection
            .send_command(Verbose::new("Route not found".to_owned()))
            .await?;
        Ok(())
    }
}
