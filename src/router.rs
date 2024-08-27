use tokio::net::TcpStream;
use tracing::{error, event, info, trace, warn, Level};
use url::Url;

use crate::*;

use self::agiparse::{AGIMessage, AGIRequestType};
use self::{handler::FallbackHandler, layer::Layer};

/// A router contains the mapping from request path to handlers
/// and contains the logic for dispatching requests.
#[derive(Debug)]
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
    pub fn route<H: AGIHandler>(mut self, location: &str, handler: H) -> Self
    where
        H: 'static,
    {
        if location.len() == 0 {
            panic!("Path must not be empty");
        };
        if location.chars().next().unwrap() != '/' {
            panic!("Path must start with a '/'");
        };
        self.routes.push((
            location.split('/').skip(1).map(|s| s.to_string()).collect(),
            Box::new(handler),
        ));
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
    pub fn fallback<H: AGIHandler>(mut self, handler: H) -> Self
    where
        H: 'static,
    {
        self.fallback = Box::new(handler);
        self
    }

    /// Add a layer(middleware) to each route that currently exists.
    pub fn layer<L: Layer>(self, layer: L) -> Self {
        return Router {
            routes: self
                .routes
                .into_iter()
                .map(|(loc, handler)| {
                    (
                        loc.clone(),
                        Box::new((layer.clone()).layer(handler)) as Box<dyn AGIHandler>,
                    )
                })
                .collect(),
            fallback: self.fallback,
        };
    }

    /// Find out, whether path defines a route that should handle url.
    ///
    /// path may contain captures and a trailing wildcard segment
    ///
    /// This function guarantees, that all defined captures have a value set in the returned
    /// hashmap
    #[tracing::instrument(level=tracing::Level::TRACE,ret)]
    fn path_matches(
        path: &Vec<String>,
        url: &Url,
    ) -> Option<(HashMap<String, String>, Option<String>)> {
        let mut idx_in_path = 0;
        let mut captures = HashMap::<String, String>::new();
        let mut wildcards = String::new();
        let path_segs_opt = url.path_segments();
        // early return for empty request path
        if path_segs_opt.is_none() {
            if path.len() == 0 {
                return Some((captures, None));
            } else {
                return None;
            };
        };
        let mut path_segs = path_segs_opt.expect("is_none should have been handled earlier");
        while let Some(segment_to_match) = path_segs.next() {
            // capture: store the value
            if path[idx_in_path].starts_with(':') {
                let name = path[idx_in_path][1..].to_string();
                captures.insert(name.to_string(), segment_to_match.to_string());
            // wildcard: match the rest of url and early return
            } else if path[idx_in_path].starts_with('*') {
                wildcards.push_str(segment_to_match);
                for rem in path_segs {
                    wildcards.push('/');
                    wildcards.push_str(rem)
                }
                return Some((captures, Some(wildcards)));
            // normal segment - simply continue iterating
            } else {
                if path[idx_in_path] != segment_to_match {
                    return None;
                }
            };
            idx_in_path += 1;
        }
        // we have iterated through the entire url that got passed to us
        // return success, if our predefined path is also exhausted
        if idx_in_path == path.len() {
            return Some((captures, None));
        } else {
            return None;
        };
    }

    /// Find the correct handler for a request.
    ///
    /// NOTE: it would be nice to remove this panic and bubble an error instead
    /// PANICS if a non-FastAGI request is passed
    #[tracing::instrument(level=tracing::Level::TRACE,ret)]
    fn route_request<'borrow>(
        &'borrow self,
        request: &AGIVariableDump,
    ) -> (
        &'borrow Box<dyn AGIHandler>,
        HashMap<String, String>,
        Option<String>,
    ) {
        let url = match &request.request {
            agiparse::AGIRequestType::FastAGI(x) => x.clone(),
            _ => {
                error!("INTERNAL ERROR. A caller to ::blazing_agi::Router::route_request must ensure that the input is FastAGI, and didn't.");
                error!("Please file an issue for this error to the blazing_agi repo");
                error!("{self:?}");
                error!("{request:?}");
                panic!("Caller must ensure that only FastAGI requests get passed.")
            }
        };
        for (path, handler) in self.routes.iter() {
            if let Some((captures, wildcards)) = Router::path_matches(path, &url) {
                return (&Box::new(handler), captures, wildcards);
            }
        }
        // nothing found. return the fallback handler
        return (&self.fallback, HashMap::<String, String>::new(), None);
    }

    /// Handle a Request.
    /// Note that differently from HTTP, a request really is an incoming stream.
    /// This function removes the protocol start from the stream, extracts some parameters
    /// and then tries to call the correct handler.
    #[tracing::instrument(level=tracing::Level::TRACE,ret)]
    pub async fn handle<'borrow>(&'borrow self, stream: TcpStream) {
        let mut conn = Connection::new(stream);

        // the first packet has to be agi_network: yes
        match conn.read_and_parse().await {
            Err(_) => {
                return;
            }
            Ok(AGIMessage::NetworkStart) => {}
            Ok(m) => {
                info!("Got incoming connection, but the first packet was not agi_network: yes");
                trace!("The packet was: {m}");
                return;
            }
        };

        // the second has to be a variable dump
        // we parse it and dispatch the correct handler
        match conn.read_and_parse().await {
            Err(_) => {
                return;
            }
            Ok(AGIMessage::VariableDump(request_data)) => {
                if let AGIRequestType::FastAGI(_) = request_data.request {
                    // find the handler responsible
                    let (handler, captures, wildcards) = self.route_request(&request_data);
                    // create the agirequest item and call the handler
                    let full_request = AGIRequest {
                        variables: request_data,
                        captures,
                        wildcards,
                    };
                    let handle_response = handler.handle(&mut conn, &full_request).await;
                    match handle_response {
                        Err(AGIError::ClientSideError(x)) => {
                            info!("During a handler, the client made an error and the handler has asked to terminate the session. The error was: {x}");
                            return;
                        }
                        Err(_) => {
                            warn!("Got a well-formed AGI request, but the handler failed. Request: {full_request:?}.");
                            return;
                        }
                        Ok(_) => {
                            event!(Level::DEBUG, "Succesfully handled a connection.");
                        }
                    };
                } else {
                    info!("Got a non-FastAGI request and ignored it.");
                    trace!("The packet was: {request_data}");
                    return;
                };
            }
            Ok(m) => {
                info!("The second packet in an incoming connection was not an AGIVariableDump. Dropping the connection.");
                trace!("The packet was: {m}");
                return;
            }
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn path_matches_simple() {
        let input_url = Url::parse("agi://some.host:4573/some/route").unwrap();
        let known_path = vec!["some".to_string(), "route".to_string()];
        assert_eq!(
            Router::path_matches(&known_path, &input_url),
            Some((HashMap::<String, String>::new(), None))
        );
    }

    #[test]
    fn path_matches_wildcards() {
        let input_url = Url::parse("agi://some.host:4573/some/route/appended/wildcard").unwrap();
        let known_path = vec![
            "some".to_string(),
            "route".to_string(),
            "*irrelevant".to_string(),
        ];
        assert_eq!(
            Router::path_matches(&known_path, &input_url),
            Some((
                HashMap::<String, String>::new(),
                Some("appended/wildcard".to_string())
            ))
        );
    }

    #[test]
    fn path_matches_empty_wildcard() {
        let input_url = Url::parse("agi://some.host:4573/some/route").unwrap();
        let known_path = vec!["some".to_string(), "route".to_string(), "*".to_string()];
        assert_eq!(Router::path_matches(&known_path, &input_url), None);
    }

    #[test]
    fn path_matches_trivial_wildcard() {
        let input_url = Url::parse("agi://some.host:4573/some/route/").unwrap();
        let known_path = vec!["some".to_string(), "route".to_string(), "*".to_string()];
        assert_eq!(
            Router::path_matches(&known_path, &input_url),
            Some((HashMap::<String, String>::new(), Some("".to_string())))
        );
    }

    #[test]
    fn path_matches_captures() {
        let input_url = Url::parse("agi://some.host:4573/scripts/the_script").unwrap();
        let known_path = vec!["scripts".to_string(), ":name".to_string()];
        let mut expect_captures = HashMap::<String, String>::new();
        expect_captures.insert("name".to_string(), "the_script".to_string());
        assert_eq!(
            Router::path_matches(&known_path, &input_url),
            Some((expect_captures, None))
        );
    }

    #[test]
    fn path_matches_captures_and_wildcard() {
        let input_url = Url::parse("agi://some.host:4573/scripts/the_script/additionals").unwrap();
        let known_path = vec![
            ":directory".to_string(),
            ":name".to_string(),
            "*".to_string(),
        ];
        let mut expect_captures = HashMap::<String, String>::new();
        expect_captures.insert("directory".to_string(), "scripts".to_string());
        expect_captures.insert("name".to_string(), "the_script".to_string());
        assert_eq!(
            Router::path_matches(&known_path, &input_url),
            Some((expect_captures, Some("additionals".to_string())))
        );
    }

    #[test]
    fn path_matches_trivial_path_segments() {
        let input_url = Url::parse("agi://some.host:4573/scripts//").unwrap();
        let known_path = vec![
            ":directory".to_string(),
            ":name".to_string(),
            "".to_string(),
        ];
        let mut expect_captures = HashMap::<String, String>::new();
        expect_captures.insert("directory".to_string(), "scripts".to_string());
        expect_captures.insert("name".to_string(), "".to_string());
        assert_eq!(
            Router::path_matches(&known_path, &input_url),
            Some((expect_captures, None))
        );
    }

    #[test]
    fn path_matches_empty_path() {
        let input_url = Url::parse("agi://some.host:4573").unwrap();
        let known_path = vec![];
        let expect_captures = HashMap::<String, String>::new();
        assert_eq!(
            Router::path_matches(&known_path, &input_url),
            Some((expect_captures, None))
        );
    }

    #[test]
    fn path_matches_no_match() {
        let input_url = Url::parse("agi://some.host:4573/some/path").unwrap();
        let known_path = vec!["other_path".to_string()];
        assert_eq!(Router::path_matches(&known_path, &input_url), None);
    }
}
