#![allow(dead_code)]

use crate::data::state::ClientState;
use ehttp::{Credentials, Headers, Request, Response};
use log::{debug, error};
use pigweb_common::bulk::{BulkImport, BulkPatch, BulkQuery};
use pigweb_common::pigs::{Pig, PigQuery};
use pigweb_common::users::{Roles, User, UserFetchResponse, UserQuery};
use pigweb_common::{query, yuri, AUTH_API_ROOT, BULK_API_ROOT, PIG_API_ROOT, USER_API_ROOT};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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
type MaybeWaiting<T> = Option<Receiver<Result<T, ApiError>>>;

/// Represents the status of a request
pub enum Status<T> {
    /// The request is done, here's the value
    Received(T),

    /// There was a problem taking care of this
    Errored(ApiError),

    /// We haven't received a response from the Sender for whatever reason
    Pending,
}

/// When Rocket returns an HTTP error as JSON, the actual error data is wrapped
/// in an "error" tag. This represents the parent tag, with ApiError holding the
/// data we actually care about.
#[derive(Debug, Deserialize)]
struct ApiErrorWrapper {
    error: ApiError,
}

/// Represents an error encountered when handling API requests
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiError {
    /// The HTTP code returned by the server. Not set for local errors (JSON parsing)
    pub code: Option<u16>,

    /// The "Reason" the error occurred
    pub reason: Option<String>,

    /// A brief description of what the error is
    pub description: String,
}

impl ApiError {
    /// Creates a new ApiError with the given description
    pub fn new(description: String) -> Self {
        Self { code: None, reason: None, description }
    }

    /// Sets the HTTP status code to the given value
    pub fn with_code(mut self, code: u16) -> Self {
        self.code = Some(code);
        self
    }

    /// Sets the short reason the error occurred, used as the title
    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }
}

/// Helper to get ApiErrors from Responses
impl From<Response> for ApiError {
    fn from(res: Response) -> Self {
        res.json::<ApiErrorWrapper>()
            .map_err(|err| ApiErrorWrapper { error: std::io::Error::from(err).into() })
            .unwrap_or_else(|e| e)
            .error
    }
}

/// serde_json::Errors can be converted into std::io::Errors. This makes it easy
/// to convert a JSON parse error into an error we care about.
impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        Self { code: None, reason: Some(err.kind().to_string()), description: err.to_string() }
    }
}

/// Defines an individual API endpoint handler. Each handler has the following
/// functions:
/// - `request(input)` submits a request to the API
/// - `resolve()` checks whether the request received a response and returns it
/// - `received(state)` returns the response value as an option and performs
///   default error handling if something went wrong (shows a modal)
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
/// - The expression actually making the request, should return a [`Receiver`]
///
/// Example:
/// ```rust
/// endpoint!(PigDeleteHandler, Uuid, Response, |input: Uuid| {
///     let (tx, rx) = oneshot::channel();
///
///     // Convert method type to DELETE, ::get method is just a good starter
///     let req = Request {
///         method: "DELETE".to_owned(),
///         credentials: Credentials::SameOrigin,
///         headers: Headers::new(&[("Accept", "application/json"), ("Content-Type", "text/plain; charset=utf-8")]),
///         ..Request::get(yuri!(PIG_API_ROOT, "delete" ;? query!("id" = input.to_string().as_str())))
///     };
///
///     // Submit the request, no fancy processing needed for this one
///     fetch_and_send(req, tx, |res| {
///         // Handle errors
///         if res.status >= 400 {
///             return Err(res.into());
///         }
///
///         Ok(res)
///     });
///
///     rx
/// });
/// ```
// this must defined BEFORE the individual endpoints
macro_rules! endpoint {
    ($name:ident, $input:ty, $output:ty, $requester:expr) => {
        #[derive(Debug)]
        pub struct $name {
            receiver: MaybeWaiting<$output>,
        }

        impl Default for $name {
            fn default() -> Self {
                Self { receiver: None }
            }
        }

        impl $name {
            /// Submit a request with the given input to this endpoint
            pub fn request(&mut self, input: $input) {
                self.receiver = Some($requester(input));
            }

            /// Returns Some if the endpoint gave a successful response.
            ///
            /// If resolve() returns error 401, clears the user's session and
            /// forces them to sign in again. Displays any other error.
            ///
            /// No action is taken if the status is still pending.
            pub fn received(&mut self, state: &mut ClientState) -> Option<$output> {
                match self.resolve() {
                    Status::Received(res) => Some(res),
                    Status::Errored(err) => {
                        if err.code == Some(401) {
                            state.authorized = None;
                        } else {
                            state.pages.layout.display_error.push(err);
                        }
                        None
                    }
                    Status::Pending => None,
                }
            }

            /// Returns the status of the last request sent to this endpoint.
            pub fn resolve(&mut self) -> Status<$output> {
                let status = check_response_status(&mut self.receiver);

                // Drop the receiver if we have a response
                if !matches!(&status, crate::data::api::Status::Pending) {
                    self.discard();
                }

                status
            }

            /// Cancels the current request to this endpoint, ignoring any
            /// response.
            pub fn discard(&mut self) {
                self.receiver = None;
            }
        }
    };
}

/// API for the current user's session and permissions
#[derive(Debug, Default)]
pub struct AuthApi {
    /// If the user is signed in, returns a list of roles, otherwise [None]
    pub is_authenticated: AuthCheckHandler,
}

endpoint!(AuthCheckHandler, bool, Option<BTreeSet<Roles>>, |_ignored: bool| {
    let (tx, rx) = oneshot::channel();

    // Submit the request to the server
    let req = Request {
        credentials: Credentials::SameOrigin,
        headers: Headers::new(&[("Accept", "application/json")]),
        ..Request::get(yuri!(AUTH_API_ROOT))
    };

    fetch_and_send(req, tx, |res| {
        if res.ok {
            return res
                .json::<BTreeSet<Roles>>() // try to parse response into JSON
                .map(|roles| Some(roles)) // if JSON parsed successfully, turn it into an option
                .map_err(|err| std::io::Error::from(err).into()); // return JSON parse error
        } else if res.status == 401 {
            return Ok(None);
        }

        Err(res.json::<ApiError>().unwrap_or_else(|err| std::io::Error::from(err).into()))
    });

    rx
});

/// The API for importing multiple names at a time
#[derive(Debug, Default)]
pub struct BulkApi {
    /// Creates a new import from the given list of names
    pub create: BulkCreateHandler,

    /// Applies the given changes to the import, returning the changes for
    /// convenience upon success
    pub patch: BulkPatchHandler,

    /// Fetches all imports which the user can access and matches the given
    /// query
    pub fetch: BulkFetchHandler,
}

endpoint!(BulkCreateHandler, &Vec<String>, BulkImport, |input| {
    let (tx, rx) = oneshot::channel();

    // If the JSON POST request was generated successfully
    let req = Request::json(yuri!(BULK_API_ROOT, "create"), input);
    if let Ok(req) = req {
        // Add correct options to the request
        let req = Request {
            credentials: Credentials::SameOrigin,
            headers: Headers::new(&[("Accept", "application/json"), ("Content-Type", "text/plain; charset=utf-8")]),
            ..req
        };

        // Now actually submit the request, then relay the result to the channel sender
        fetch_and_send(req, tx, |res| {
            // Handle errors
            if res.status >= 400 {
                return Err(res.into());
            }

            // Convert the response to the correct type
            res.json::<BulkImport>().map_err(|err| std::io::Error::from(err).into())
        });
    } else {
        tx.send(Err(std::io::Error::from(req.unwrap_err()).into())).unwrap_or_default()
    }

    rx
});

endpoint!(BulkPatchHandler, BulkPatch, BulkPatch, |input: BulkPatch| {
    let (tx, rx) = oneshot::channel();

    // If the JSON POST request was generated successfully
    let req = Request::json(yuri!(BULK_API_ROOT, "patch"), &input);
    if let Ok(req) = req {
        // Add correct options to the request
        let req = Request {
            method: "PATCH".to_owned(),
            credentials: Credentials::SameOrigin,
            headers: Headers::new(&[("Accept", "application/json"), ("Content-Type", "text/plain; charset=utf-8")]),
            ..req
        };

        // Now actually submit the request, then relay the result to the channel sender
        // No fancy processing needed for this one
        fetch_and_send(req, tx, |res| {
            // Handle errors
            if res.status >= 400 {
                return Err(res.into());
            }

            Ok(input)
        });
    } else {
        tx.send(Err(std::io::Error::from(req.unwrap_err()).into())).unwrap_or_default()
    }

    rx
});

endpoint!(BulkFetchHandler, &BulkQuery, Vec<BulkImport>, |input: &BulkQuery| {
    let (tx, rx) = oneshot::channel();

    // Submit the request to the server
    let req = Request {
        credentials: Credentials::SameOrigin,
        headers: Headers::new(&[("Accept", "application/json")]),
        ..Request::get(input.to_yuri())
    };
    fetch_and_send(req, tx, |res| {
        // Handle errors
        if res.status >= 400 {
            return Err(res.into());
        }

        // Convert the response to the correct type
        res.json::<Vec<BulkImport>>().map_err(|err| std::io::Error::from(err).into())
    });

    rx
});

/// The API for working with pigs
#[derive(Debug, Default)]
pub struct PigApi {
    /// Create a new pig given the name as a &str
    pub create: PigCreateHandler,

    /// Update a pig given the updated Pig struct
    pub update: PigUpdateHandler,

    /// Delete a pig given the Uuid
    pub delete: PigDeleteHandler,

    /// Searches for pigs baesd on the given &str query
    pub fetch: PigFetchHandler,
}

endpoint!(PigCreateHandler, &str, Pig, |input| {
    let (tx, rx) = oneshot::channel();

    // Submit the request to the server
    let req = Request {
        credentials: Credentials::SameOrigin,
        headers: Headers::new(&[("Accept", "application/json"), ("Content-Type", "text/plain; charset=utf-8")]),
        ..Request::post(yuri!(PIG_API_ROOT, "create" ;? query!("name" = input)), vec![])
    };
    fetch_and_send(req, tx, |res| {
        // Handle errors
        if res.status >= 400 {
            return Err(res.into());
        }

        // Convert the response to a pig object
        res.json::<Pig>().map_err(|err| std::io::Error::from(err).into())
    });

    rx
});

endpoint!(PigUpdateHandler, &Pig, Response, |input| {
    let (tx, rx) = oneshot::channel();

    // If the JSON POST was generated successfully
    let req = Request::json(yuri!(PIG_API_ROOT, "update"), input);
    if let Ok(req) = req {
        // Convert the request type from POST to PUT
        let req = Request {
            method: "PUT".to_owned(),
            credentials: Credentials::SameOrigin,
            headers: Headers::new(&[("Accept", "application/json"), ("Content-Type", "application/json")]),
            ..req
        };

        // Now actually submit the request, then relay the result to the channel sender
        // No fancy processing needed for this one
        fetch_and_send(req, tx, |res| {
            // Handle errors
            if res.status >= 400 {
                return Err(res.into());
            }

            Ok(res)
        });
    } else {
        tx.send(Err(std::io::Error::from(req.unwrap_err()).into())).unwrap_or_default()
    }

    rx
});

endpoint!(PigDeleteHandler, Uuid, Response, |input: Uuid| {
    let (tx, rx) = oneshot::channel();

    // Convert method type to DELETE, ::get method is just a good starter
    let req = Request {
        method: "DELETE".to_owned(),
        credentials: Credentials::SameOrigin,
        headers: Headers::new(&[("Accept", "application/json"), ("Content-Type", "text/plain; charset=utf-8")]),
        ..Request::get(yuri!(PIG_API_ROOT, "delete" ;? query!("id" = input.to_string().as_str())))
    };

    // Submit the request, no fancy processing needed for this one
    fetch_and_send(req, tx, |res| {
        // Handle errors
        if res.status >= 400 {
            return Err(res.into());
        }

        Ok(res)
    });

    rx
});

endpoint!(PigFetchHandler, PigQuery, Vec<Pig>, |params: PigQuery| {
    let (tx, rx) = oneshot::channel();

    // Submit the request to the server
    let req = Request {
        credentials: Credentials::SameOrigin,
        headers: Headers::new(&[("Accept", "application/json")]),
        ..Request::get(params.to_yuri())
    };
    fetch_and_send(req, tx, |res| {
        // Handle errors
        if res.status >= 400 {
            return Err(res.into());
        }

        // Convert the response to a pig object
        res.json::<Vec<Pig>>().map_err(|err| std::io::Error::from(err).into())
    });

    rx
});

/// The API for working with users
#[derive(Debug, Default)]
pub struct UserApi {
    /// Fetch a list of user structs--or a mapping of their uuids to usernames,
    /// based on permissions--which fit the query
    pub fetch: UserFetchHandler,

    /// Fetch a list of roles for each user which fits the query
    pub roles: UserRolesHandler,

    /// Expires the user with the given id and returns the updated user
    pub expire: UserExpireHandler,
}

endpoint!(UserFetchHandler, UserQuery, UserFetchResponse, |params: UserQuery| {
    let (tx, rx) = oneshot::channel();

    // Submit the request to the server
    let req = Request {
        credentials: Credentials::SameOrigin,
        headers: Headers::new(&[("Accept", "application/json")]),
        ..Request::get(params.to_yuri())
    };
    fetch_and_send(req, tx, |res| {
        // Handle errors
        if res.status >= 400 {
            return Err(res.into());
        }

        // Convert the response to the struct
        res.json::<UserFetchResponse>().map_err(|err| std::io::Error::from(err).into())
    });

    rx
});

endpoint!(UserRolesHandler, UserQuery, BTreeMap<Uuid, BTreeSet<Roles>>, |params: UserQuery| {
    let (tx, rx) = oneshot::channel();

    // Submit the request to the server
    let req = Request {
        credentials: Credentials::SameOrigin,
        headers: Headers::new(&[("Accept", "application/json")]),
        ..Request::get(yuri!(USER_API_ROOT, "roles" ;? query!(params)))
    };
    fetch_and_send(req, tx, |res| {
        // Handle errors
        if res.status >= 400 {
            return Err(res.into());
        }

        // Convert the response to the map
        res.json::<BTreeMap<Uuid, BTreeSet<Roles>>>().map_err(|err| std::io::Error::from(err).into())
    });

    rx
});

endpoint!(UserExpireHandler, Uuid, User, |input: Uuid| {
    let (tx, rx) = oneshot::channel();

    // Convert method type to PATCH, ::get method is just a good starter
    let req = Request {
        method: "PATCH".to_owned(),
        credentials: Credentials::SameOrigin,
        headers: Headers::new(&[("Accept", "application/json"), ("Content-Type", "text/plain; charset=utf-8")]),
        ..Request::get(yuri!(USER_API_ROOT, "expire" ;? query!("id" = input.to_string().as_str())))
    };

    // Submit the request, no fancy processing needed for this one
    fetch_and_send(req, tx, |res| {
        // Handle errors
        if res.status >= 400 {
            return Err(res.into());
        }

        // Convert the response to a user
        res.json::<User>().map_err(|err| std::io::Error::from(err).into())
    });

    rx
});

/// Submits the given request, then if successful, processes the on_response
/// callback and submits the return value from it to the tx channel sender.
fn fetch_and_send<T: 'static + Send>(
    req: Request,
    tx: Sender<Result<T, ApiError>>,
    on_response: impl 'static + Send + FnOnce(Response) -> Result<T, ApiError>,
) {
    debug!("Sending request: {req:?}\nBody: {}", String::from_utf8(req.body.clone()).unwrap_or_default());

    // No fancy processing needed for this one
    ehttp::fetch(req, |result| {
        tx.send(match result {
            Ok(res) => {
                debug!("Received response: {res:?}\nBody: {}", res.text().unwrap_or_default());
                on_response(res)
            }
            Err(msg) => {
                // when we reach this branch, it's *usually* that we didn't get a response.
                // HTTP error codes are handled by the success branch here.
                error!("Encountered fetch error: {:?}", msg.to_owned());
                Err(ApiError::new(msg.to_owned()).with_reason("No response".to_owned()))
            }
        })
        .unwrap_or_default()
    });
}

/// Determines the status of a submitted request
fn check_response_status<T>(maybe: &mut MaybeWaiting<T>) -> Status<T> {
    match maybe {
        // we have a request to check up on
        Some(receiver) => match receiver.try_recv() {
            // the channel got a response from the request
            Ok(res) => match res {
                // response was successful
                Ok(t) => Status::Received(t),
                // there was an error
                Err(e) => Status::Errored(e),
            },
            // we're still waiting on a response
            Err(_) => Status::Pending,
        },
        // we're not waiting on any request
        None => Status::Pending,
    }
}
