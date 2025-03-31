use crate::data::Status::{Errored, Pending, Received};
use ehttp::{Credentials, Request, Response};
use log::debug;
use pigweb_common::pigs::{Pig, PigFetchQuery};
use pigweb_common::{query, yuri, PIG_API_ROOT};
use tokio::sync::oneshot;
use tokio::sync::oneshot::{Receiver, Sender};
use uuid::Uuid;

/// Utility type to represent a result we may be waiting on. Named because we
/// may or may not have a receiver waiting on the result.
///
/// These are Options so that when the value in the receiver/result is read, we
/// can revert back to None. This means we're not beating a dead horse every
/// frame and wasting cycles reassigning data in app.rs that hasn't changed AND
/// we get to free up the memory.
type MaybeWaiting<T> = Option<Receiver<Result<T, String>>>;

/// Represents the status of a request
pub enum Status<T> {
    /// The request is done, here's the value
    Received(T),

    /// There was a problem taking care of this
    Errored(String),

    /// We haven't received a response from the Sender for whatever reason
    Pending,
}

/// Defines an individual API endpoint handler. Each handler has three methods:
/// - `request(input)` submits a request to the API
/// - `resolve()` checks whether the request received a response and returns it
/// - `discard()` forgets the previous request which was made
///
/// This is designed around immediate-mode GUIs or anything which needs to be
/// refreshed constantly and where you only care about the last thing submitted
/// to the server.
///
/// Implementing this macro requires four parameters:
/// - The name of the handler struct
/// - The input type expected when making a request
/// - The output type expected from the server
/// - The expression actually making the request, should return a tokio::sync::oneshot::Receiver
// this must defined BEFORE the individual endpoints
macro_rules! endpoint {
    ($name:ident, $input:ty, $output:ty, $requester:expr) => {
        pub struct $name {
            receiver: MaybeWaiting<$output>,
        }

        impl Default for $name {
            fn default() -> Self {
                Self { receiver: None }
            }
        }

        impl $name {
            pub fn request(&mut self, input: $input) {
                self.receiver = Some($requester(input));
            }

            pub fn resolve(&mut self) -> Status<$output> {
                let status = check_response_status(&mut self.receiver);

                // Drop the receiver if we have a response
                if !matches!(&status, Pending) {
                    self.discard();
                }

                status
            }

            pub fn discard(&mut self) {
                self.receiver = None;
            }
        }
    };
}

pub struct PigApi {
    pub create: PigCreateHandler,
    pub update: PigUpdateHandler,
    pub delete: PigDeleteHandler,
    pub fetch: PigFetchHandler,
}

impl Default for PigApi {
    fn default() -> Self {
        // These must be defined individually or else we run into a "too much recursion" error
        Self {
            create: PigCreateHandler::default(),
            update: PigUpdateHandler::default(),
            delete: PigDeleteHandler::default(),
            fetch: PigFetchHandler::default(),
        }
    }
}

endpoint!(PigCreateHandler, &str, Pig, |input| {
    let (tx, rx) = oneshot::channel();

    // Submit the request to the server
    let req = Request {
        credentials: Credentials::SameOrigin,
        ..Request::post(yuri!(PIG_API_ROOT, "create" ;? query!("name" = input)), vec![])
    };
    fetch_and_send(req, tx, |res| {
        // Convert the response to a pig object
        let json = res.json::<Pig>();

        // Check if the JSON parsed successfully, both times we don't
        // care whether the message is received by rx
        match json {
            Ok(pig) => Ok(pig),
            Err(err) => Err(format!("Unable to parse JSON: {}", err.to_string())),
        }
    });

    rx
});

endpoint!(PigUpdateHandler, &Pig, Response, |input| {
    let (tx, rx) = oneshot::channel();

    // If the JSON POST was generated successfully
    let req = Request::json(yuri!(PIG_API_ROOT, "update"), input);
    if let Ok(req) = req {
        // Convert the request type from POST to PUT
        let req = Request { method: "PUT".to_owned(), credentials: Credentials::SameOrigin, ..req };

        // Now actually submit the request, then relay the result to the channel sender
        // No fancy processing needed for this one
        fetch_and_send(req, tx, |res| Ok(res));
    } else {
        tx.send(Err(format!("Unable to generate JSON: {}", req.unwrap_err().to_string()))).unwrap_or_default()
    }

    rx
});

endpoint!(PigDeleteHandler, Uuid, Response, |input: Uuid| {
    let (tx, rx) = oneshot::channel();

    // Convert method type to DELETE, ::get method is just a good starter
    let req = Request {
        method: "DELETE".to_owned(),
        credentials: Credentials::SameOrigin,
        ..Request::get(yuri!(PIG_API_ROOT, "delete" ;? query!("id" = input.to_string().as_str())))
    };

    // Submit the request, no fancy processing needed for this one
    fetch_and_send(req, tx, |res| Ok(res));

    rx
});

endpoint!(PigFetchHandler, &str, Vec<Pig>, |input: &str| {
    let (tx, rx) = oneshot::channel();

    // Submit the request to the server
    let params = PigFetchQuery { name: Some(input.to_owned()), ..Default::default() };
    let req = Request { credentials: Credentials::SameOrigin, ..Request::get(params.to_yuri()) };
    fetch_and_send(req, tx, |res| {
        // Convert the response to a pig object
        let json = res.json::<Vec<Pig>>();

        // Check if the JSON parsed successfully, both times we don't
        // care whether the message is received by rx
        match json {
            Ok(pigs) => Ok(pigs),
            Err(err) => Err(format!("Unable to parse JSON: {}", err.to_string())),
        }
    });

    rx
});

// TODO use std::error::Error instead of strings for responses

fn fetch_and_send<T: 'static + Send>(
    req: Request,
    tx: Sender<Result<T, String>>,
    on_response: impl 'static + Send + FnOnce(Response) -> Result<T, String>,
) {
    debug!("Sending request: {req:?}\nBody: {}", String::from_utf8(req.body.clone()).unwrap_or_default());

    // No fancy processing needed for this one
    ehttp::fetch(req, |result| {
        tx.send(match result {
            Ok(res) => {
                debug!("Received response: {res:?}\nBody: {}", res.text().unwrap_or_default());
                on_response(res)
            }
            Err(msg) => Err(format!("No response: {}", msg.to_owned())),
        })
        .unwrap_or_default()
    });
}

fn check_response_status<T>(maybe: &mut MaybeWaiting<T>) -> Status<T> {
    match maybe {
        Some(receiver) => match receiver.try_recv() {
            Ok(res) => match res {
                Ok(t) => Received(t),
                Err(msg) => Errored(msg),
            },
            Err(_) => Pending,
        },
        None => Pending,
    }
}
