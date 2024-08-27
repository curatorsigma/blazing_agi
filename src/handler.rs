use crate::{AGICommand, AGIError, AGIRequest, Connection};

#[async_trait::async_trait]
pub trait AGIHandler: Send + Sync {
    async fn handle(&self, connection: &mut Connection, request: &AGIRequest) -> Result<(), AGIError>;
}

#[async_trait::async_trait]
impl AGIHandler for Box<dyn AGIHandler> {
    async fn handle(&self, conn: &mut Connection, req: &AGIRequest) -> Result<(), AGIError>
    {
        (**self).handle(conn, req).await
    }
}

#[async_trait::async_trait]
impl AGIHandler for &Box<dyn AGIHandler> {
    async fn handle(&self, conn: &mut Connection, req: &AGIRequest) -> Result<(), AGIError>
    {
        (**self).handle(conn, req).await
    }
}

#[async_trait::async_trait]
impl AGIHandler for &dyn AGIHandler {
    async fn handle(&self, conn: &mut Connection, req: &AGIRequest) -> Result<(), AGIError>
    {
        (**self).handle(conn, req).await
    }
}

pub struct AndThenHandler {
    first: Box<dyn AGIHandler>,
    second: Box<dyn AGIHandler>,
}
#[async_trait::async_trait]
impl AGIHandler for AndThenHandler {
    async fn handle(&self, connection: &mut Connection, request: &AGIRequest) -> Result<(), AGIError>
    {
        self.first.handle(connection, request).await?;
        self.second.handle(connection, request).await
    }
}

/// A trivial AGI response, simply acknowledging that a route does not exist.
pub(crate) struct FallbackHandler {}
#[async_trait::async_trait]
impl AGIHandler for FallbackHandler {
    async fn handle(&self, connection: &mut Connection, _: &AGIRequest) -> Result<(), AGIError>
    {
        connection
            .send_command(AGICommand::Verbose("Route not found".to_string()))
            .await?;
        Ok(())
    }
}

