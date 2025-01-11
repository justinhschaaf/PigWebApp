use ehttp::{Request, Response};
use form_urlencoded::byte_serialize;
use pigweb_common::Pig;
use poll_promise::Promise;
use uuid::Uuid;

pub struct ClientDataHandler {
    // PIG API - expected responses
    pig_create_response: PromisedResponse,
    pig_update_response: PromisedResponse,
    pig_delete_response: PromisedResponse,
    pig_fetch_response: PromisedResponse,
}

type PromisedResponse = Option<Promise<Result<Response, String>>>;

impl Default for ClientDataHandler {
    fn default() -> Self {
        Self {
            pig_create_response: None,
            pig_update_response: None,
            pig_delete_response: None,
            pig_fetch_response: None,
        }
    }
}

impl ClientDataHandler {
    // PIG API
    pub fn request_pig_create(&mut self, name: &str) {
        self.pig_create_response = Some(Promise::spawn_local({
            // Encode special characters https://rustjobs.dev/blog/how-to-url-encode-strings-in-rust/
            let encoded_name = byte_serialize(name.as_bytes()).collect();

            // Return the result from making the request
            let req = Request::post(format!("/api/pigs/create?name={}", encoded_name), vec![]);
            ehttp::fetch_async(req)
        }));
    }

    pub fn resolve_pig_create(&mut self, ok: impl FnOnce(Pig), err: impl FnOnce(&String)) {
        handle_promised_response(
            self.pig_create_response.as_mut(),
            |res| {
                self.pig_create_response = None;
                let json = res.json::<Pig>();
                match json {
                    Ok(pig) => ok(pig),
                    Err(e) => err(&e.to_string()),
                }
            },
            |msg| err(msg),
        );
    }

    pub fn discard_pig_create(&mut self) {
        self.pig_create_response = None;
    }

    pub fn request_pig_update(&mut self, pig: &Pig) {
        self.pig_update_response = Some(Promise::spawn_local({
            // Create a request object, we need a match incase JSON fails
            match Request::json("/api/pigs/update", pig) {
                // Send the request if we can
                Ok(req) => ehttp::fetch_async(Request {
                    // Convert the request type from POST to PUT
                    method: "PUT".to_owned(),
                    ..req
                }),
                Err(e) => Err(e.to_string()),
            }
        }));
    }

    pub fn resolve_pig_update(&mut self, ok: impl FnOnce(&Response), err: impl FnOnce(&String)) {
        handle_promised_response(
            self.pig_update_response.as_mut(),
            |res| {
                self.pig_update_response = None;
                ok(res);
            },
            |msg| err(msg),
        );
    }

    pub fn discard_pig_update(&mut self) {
        self.pig_update_response = None;
    }

    pub fn request_pig_delete(&mut self, id: Uuid) {
        self.pig_delete_response = Some(Promise::spawn_local({
            ehttp::fetch_async(Request {
                // Convert method type to DELETE, ::get method is just a good starter
                method: "DELETE".to_owned(),
                ..Request::get(format!("/api/pigs/delete?id={}", id.to_string()))
            })
        }));
    }

    pub fn resolve_pig_delete(&mut self, ok: impl FnOnce(&Response), err: impl FnOnce(&String)) {
        handle_promised_response(
            self.pig_delete_response.as_mut(),
            |res| {
                self.pig_delete_response = None;
                ok(res);
            },
            |msg| err(msg),
        );
    }

    pub fn discard_pig_delete(&mut self) {
        self.pig_delete_response = None;
    }

    pub fn request_pig_fetch(&mut self, query: &str) {
        self.pig_fetch_response = Some(Promise::spawn_local({
            // Encode special characters https://rustjobs.dev/blog/how-to-url-encode-strings-in-rust/
            let encoded_name = byte_serialize(query.as_bytes()).collect();

            // Return the result from making the request
            // TODO replace string URLs with ones from the url crate, we have it might as well use it
            let req = Request::post(format!("/api/pigs/fetch?name={}", encoded_name), vec![]);
            ehttp::fetch_async(req)
        }));
    }

    pub fn resolve_pig_fetch(&mut self, ok: impl FnOnce(Vec<Pig>), err: impl FnOnce(&String)) {
        handle_promised_response(
            self.pig_fetch_response.as_mut(),
            |res| {
                self.pig_fetch_response = None;
                let json = res.json::<Vec<Pig>>();
                match json {
                    Ok(pigs) => ok(pigs),
                    Err(e) => err(&e.to_string()),
                }
            },
            |msg| err(msg),
        );
    }

    pub fn discard_pig_fetch(&mut self) {
        self.pig_fetch_response = None;
    }
}

/// Utility method to avoid ugly nested code. Checks if the given option has a
/// promise, and if so, checks whether the promise is ready. If that also passes
/// it runs the provided callbacks based on the result
fn handle_promised_response(
    optional_promise: Option<&mut Promise<Result<Response, String>>>,
    ok: impl FnOnce(&Response),
    err: impl FnOnce(&String),
) {
    if optional_promise.is_some() {
        if let Some(result) = optional_promise.unwrap().ready() {
            match result {
                Ok(res) => {
                    if res.ok {
                        ok(res);
                    } else {
                        err(&res.status_text)
                    }
                }
                Err(msg) => err(msg),
            }
        }
    }
}
