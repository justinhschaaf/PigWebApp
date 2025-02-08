use crate::data::Status::{Errored, Pending, Received};
use ehttp::{Request, Response};
use log::debug;
use pigweb_common::{query, yuri, Pig, PigFetchQuery, PIG_API_ROOT};
use tokio::sync::oneshot;
use tokio::sync::oneshot::{Receiver, Sender};
use uuid::Uuid;

pub struct ClientDataHandler {
    // PIG API
    pig_create_receiver: MaybeWaiting<Pig>,
    pig_update_receiver: MaybeWaiting<Response>,
    pig_delete_receiver: MaybeWaiting<Response>,
    pig_fetch_receiver: MaybeWaiting<Vec<Pig>>,
}

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

impl Default for ClientDataHandler {
    fn default() -> Self {
        Self {
            pig_create_receiver: None,
            pig_update_receiver: None,
            pig_delete_receiver: None,
            pig_fetch_receiver: None,
        }
    }
}

impl ClientDataHandler {
    // PIG API
    pub fn request_pig_create(&mut self, name: &str) {
        // Encode special characters https://rustjobs.dev/blog/how-to-url-encode-strings-in-rust/
        let (tx, rx) = oneshot::channel();
        self.pig_create_receiver = Some(rx);

        // Submit the request to the server
        let req = Request::post(yuri!(PIG_API_ROOT, "create" ;? query!("name" = name)), vec![]);
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
    }

    pub fn resolve_pig_create(&mut self) -> Status<Pig> {
        let status = check_response_status(&mut self.pig_create_receiver);

        // Drop the receiver if we have a response
        if !matches!(&status, Pending) {
            self.pig_create_receiver = None;
        }

        status
    }

    pub fn discard_pig_create(&mut self) {
        self.pig_create_receiver = None;
    }

    pub fn request_pig_update(&mut self, pig: &Pig) {
        let (tx, rx) = oneshot::channel();
        self.pig_update_receiver = Some(rx);

        // If the JSON POST was generated successfully
        let req = Request::json(yuri!(PIG_API_ROOT, "update"), pig);
        if let Ok(req) = req {
            // Convert the request type from POST to PUT
            let req = Request { method: "PUT".to_owned(), ..req };

            // Now actually submit the request, then relay the result to the channel sender
            // No fancy processing needed for this one
            fetch_and_send(req, tx, |res| Ok(res));
        } else {
            tx.send(Err(format!("Unable to generate JSON: {}", req.unwrap_err().to_string()))).unwrap_or_default()
        }
    }

    pub fn resolve_pig_update(&mut self) -> Status<Response> {
        let status = check_response_status(&mut self.pig_update_receiver);

        // Drop the receiver if we have a response
        if !matches!(&status, Pending) {
            self.pig_update_receiver = None;
        }

        status
    }

    pub fn discard_pig_update(&mut self) {
        self.pig_update_receiver = None;
    }

    pub fn request_pig_delete(&mut self, id: Uuid) {
        let (tx, rx) = oneshot::channel();
        self.pig_delete_receiver = Some(rx);

        // Convert method type to DELETE, ::get method is just a good starter
        let req = Request {
            method: "DELETE".to_owned(),
            ..Request::get(yuri!(PIG_API_ROOT, "delete" ;? query!("id" = id.to_string().as_str())))
        };

        // Submit the request, no fancy processing needed for this one
        fetch_and_send(req, tx, |res| Ok(res));
    }

    pub fn resolve_pig_delete(&mut self) -> Status<Response> {
        let status = check_response_status(&mut self.pig_delete_receiver);

        // Drop the receiver if we have a response
        if !matches!(&status, Pending) {
            self.pig_delete_receiver = None;
        }

        status
    }

    pub fn discard_pig_delete(&mut self) {
        self.pig_delete_receiver = None;
    }

    pub fn request_pig_fetch(&mut self, query: &str) {
        let (tx, rx) = oneshot::channel();
        self.pig_fetch_receiver = Some(rx);

        // Submit the request to the server
        let params = PigFetchQuery { id: None, name: Some(query.to_owned()) };
        let req = Request::get(yuri!(PIG_API_ROOT, "fetch" ;? query!(params)));
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
    }

    pub fn resolve_pig_fetch(&mut self) -> Status<Vec<Pig>> {
        let status = check_response_status(&mut self.pig_fetch_receiver);

        // Drop the receiver if we have a response
        if !matches!(&status, Pending) {
            self.pig_fetch_receiver = None;
        }

        status
    }

    pub fn discard_pig_fetch(&mut self) {
        self.pig_fetch_receiver = None;
    }
}

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
