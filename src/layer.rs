use blazing_agi_macros::and_then;
use crate::handler::AndThenHandler;

use crate::handler::AGIHandler;

/// A layer (middleware) that transforms a handler into another handler
pub trait Layer: Clone {
    fn layer<H: AGIHandler + 'static>(&self, handler: H) -> Box<dyn AGIHandler>;
}

#[derive(Clone)]
pub struct AndThenLayerBefore<I>
where I: Clone
{
    handler: Box<I>,
}
impl<I> AndThenLayerBefore<I>
where I: Clone + AGIHandler + 'static
{
    pub fn new(handler: I) -> Self {
        AndThenLayerBefore { handler: Box::new(handler) }
    }
}
impl<I> Layer for AndThenLayerBefore<I>
where I: Clone + AGIHandler + 'static
{
    fn layer<H: AGIHandler + 'static>(&self, handler: H) -> Box<dyn AGIHandler> {
        Box::new(AndThenHandler::new(
            self.handler.clone(),
            Box::new(handler)
        ))
    }
}

