use tokio::net::TcpStream;
use url::Url;

use crate::*;

use self::agiparse::{AGIMessage, AGIRequestType};
use self::{handler::FallbackHandler, layer::Layer};

/// A router contains the mapping from request path to handlers
/// and contains the logic for dispatching requests.
pub struct Router {
    routes: Vec<(Vec<String>, Box<dyn AGIHandler>)>,
    fallback: Box<dyn AGIHandler>,
}
impl Router {
    /// Create the default router which has only a simple fallback route added.
    ///
    /// It will respond to any request with "VERBOSE 'this route does not exist'"
    pub fn new() -> Self {
        Router {
            routes: vec![],
            fallback: Box::new(FallbackHandler {}),
        }
    }

    /// Add a route to this router.
    /// This is a mapping path -> handler.
    ///
    /// TODO::DOC location MUST start in '/'. It MAY contain :capture segments and end in a
    /// wildcard segment /*
    /// TODO::DOC dispatching happens from the first added route to the last added route.
    /// If one route matches, no other routes are considered.
    pub fn route<H: AGIHandler>(mut self, location: &str, handler: H) -> Self where H: 'static {
        self.routes.push((
            location.split('/').map(|s| s.to_string()).collect(),
            Box::new(handler)));
        self
    }

    /// Merge `self` with `other` router to combine routes.
    ///
    /// The fallback of the first router will be chosen, the fallback of the second ignored.
    pub fn merge(mut self, mut other: Router) -> Router {
        self.routes.append(&mut other.routes);
        self
    }

    /// Set the fallback handler.
    /// This will be called if no route matches a request.
    pub fn fallback<H: AGIHandler>(mut self, handler: H) -> Self where H: 'static {
        self.fallback = Box::new(handler);
        self
    }

    /// Add a layer(middleware) to each route that currently exists.
    pub fn layer<L: Layer>(self, layer: &L) -> Self {
        return Router {
            routes: self
                .routes
                .into_iter()
                .map(|(loc, handler)| (loc.clone(), layer.layer(handler)))
                .collect(),
            fallback: self.fallback,
        };
    }

    /// Find out, whether path defines a route that should handle url.
    ///
    /// path may contain captures and a trailing wildcard segment
    fn path_matches(path: &Vec<String>, url: &Url) -> Option<(HashMap<String, String>, Option<String>,)> {
        let mut idx_in_path = 0;
        let mut captures = HashMap::<String, String>::new();
        let mut wildcards = String::new();
        let mut path_segs = url.path_segments()?;
        while let Some(segment_to_match) = path_segs.next() {
            // capture: store the value
            if path[idx_in_path].starts_with(':') {
                let name = path[idx_in_path][1..].to_string();
                captures.insert(name.to_string(), segment_to_match.to_string());
            // wildcard: match the rest of url and early return
            } else if path[idx_in_path].starts_with('*') {
                for rem in path_segs {
                    wildcards.push_str(rem)
                };
                return Some((captures, Some(wildcards)));
            // normal segment - simply continue iterating
            } else {
                if path[idx_in_path] != segment_to_match {
                    return None;
                }
            };
            idx_in_path += 1;
        };
        // we have iterated through the entire url that got passed to us
        // return success, if our predefined path is also exhausted
        if idx_in_path == path.len() {
            return Some((captures, Some(wildcards)))
        } else {
            return None;
        };
    }

    /// Find the correct handler for a request.
    ///
    /// PANICS if a non-FastAGI request is passed
    fn route_request<'borrow>(&'borrow self, request: &AGIVariableDump) -> (&'borrow Box<dyn AGIHandler>, HashMap<String, String>, Option<String>,) {
        let url = match &request.request {
            agiparse::AGIRequestType::FastAGI(x) => {
                x.clone()
            }
            _ => { panic!("Caller must ensure that only FastAGI requests get passed.") }
        };
        for (path, handler) in self.routes.iter() {
            if let Some((captures, wildcards)) = Router::path_matches(path, &url) {
                return (&Box::new(handler), captures, wildcards);
            }
        };
        // nothing found. return the fallback handler
        return (&self.fallback, HashMap::<String, String>::new(), None);
    }

    /// Handle a Request.
    /// Note that differently from HTTP, a request really is an incoming stream.
    /// This function removes the protocol start from the stream, extracts some parameters
    /// and then tries to call the correct handler.
    pub async fn handle<'borrow>(&'borrow self, stream: TcpStream) {
        let mut conn = Connection::new(stream);

        // the first packet has to be agi_network: yes
        match conn.read_and_parse().await {
            Err(_) => { return; }
            Ok(AGIMessage::NetworkStart) => {}
            _ => { return; }
        };

        // the second has to be a variable dump
        // we parse it and dispatch the correct handler
        match conn.read_and_parse().await {
            Err(_) => { return; }
            Ok(AGIMessage::VariableDump(request_data)) => {
                if let AGIRequestType::FastAGI(_) = request_data.request {
                    // find the handler responsible
                    let (handler, captures, wildcards) = self.route_request(&request_data);
                    // create the agirequest item and call the handler
                    let full_request = AGIRequest {
                        variables: request_data,
                        captures, wildcards
                    };
                    let handle_response = handler.handle(&mut conn, &full_request).await;
                    if let Err(_) = handle_response {
                        return;
                    }
                } else {
                    return;
                };
            }
            _ => { return; }
        };
    }
}

