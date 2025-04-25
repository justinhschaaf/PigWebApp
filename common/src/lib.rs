pub mod bulk;
pub mod pigs;
pub mod users;
pub mod yuri;

#[cfg(feature = "server")]
pub mod schema;

/// The relative base URL for all authentication API routes
pub const AUTH_API_ROOT: &str = "/auth/";

/// The relative base URL for all bulk import API routes
pub const BULK_API_ROOT: &str = "/api/bulk/";

/// The relative base URL for all Pig API routes
pub const PIG_API_ROOT: &str = "/api/pigs/";

/// The relative base URL for all User API routes
pub const USER_API_ROOT: &str = "/api/users/";

#[cfg(feature = "server")]
pub const COOKIE_JWT: &str = "pigweb_jwt";

#[cfg(feature = "server")]
pub const COOKIE_USER: &str = "pigweb_user";

pub const DEFAULT_API_RESPONSE_LIMIT: u32 = 100;

/// This type is used as a type-level key for rocket_oauth2 and as the
/// cookie containing the token data.
#[cfg(feature = "server")]
pub struct OpenIDAuth;

#[cfg(feature = "server")]
pub fn parse_uuid(string: &str) -> Result<uuid::Uuid, rocket::http::Status> {
    use std::str::FromStr;
    match uuid::Uuid::from_str(string) {
        Ok(i) => Ok(i),
        Err(e) => {
            rocket::error!("Unable to parse UUID: {:?}", e);
            Err(rocket::http::Status::BadRequest)
        }
    }
}

#[cfg(feature = "server")]
pub fn parse_uuids(strings: &Vec<String>) -> Result<Vec<uuid::Uuid>, rocket::http::Status> {
    use std::str::FromStr;
    // https://stackoverflow.com/a/16756324
    match strings.iter().map(|e| uuid::Uuid::from_str(e.as_str())).collect() {
        Ok(i) => Ok(i),
        Err(e) => {
            rocket::error!("Unable to parse UUID: {:?}", e);
            Err(rocket::http::Status::BadRequest)
        }
    }
}

#[macro_export]
macro_rules! query_list {
    ($var:ident, $input:ty) => {
        // https://users.rust-lang.org/t/can-i-build-a-function-name-from-arguments-to-a-macro-rules/45061/4
        paste::item! {
            pub fn [< with_ $var >] (self, $var: &$input) -> Self {
                self.[< with_ $var s >](vec![$var.to_owned()])
            }

            pub fn [< with_ $var _string >](self, $var: &String) -> Self {
                self.[< with_ $var s_string >](vec![$var.to_owned()])
            }

            pub fn [< with_ $var s >] (self, $var: Vec<$input>) -> Self {
                self.[< with_ $var s_string >]($var.iter().map(|e| e.to_string()).collect())
            }

            pub fn [< with_ $var s_string >] (mut self, $var: Vec<String>) -> Self {
                self.$var = Some($var);
                self
            }
        }
    };
}

#[macro_export]
macro_rules! query_limit_offset {
    () => {
        pub fn with_limit(mut self, limit: u32) -> Self {
            self.limit = Some(limit);
            self
        }

        pub fn with_offset(mut self, offset: u32) -> Self {
            self.offset = Some(offset);
            self
        }
    };
}

#[macro_export]
macro_rules! query_to_yuri {
    ($segment:expr) => {
        pub fn to_yuri(&self) -> String {
            $crate::yuri!($segment, "fetch" ;? $crate::query!(self))
        }
    }
}
