use crate::handler::AGIHandler;

/// A layer (middleware) that transforms a handler into another handler
pub trait Layer {
    fn layer<H: AGIHandler>(&self, handler: H) -> H;
}
