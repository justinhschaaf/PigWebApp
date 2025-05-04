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

/// The key of the cookie storing the JWT received from the OIDC provider
#[cfg(feature = "server")]
pub const COOKIE_JWT: &str = "pigweb_jwt";

/// The key of the cookie storing the current user's info
#[cfg(feature = "server")]
pub const COOKIE_USER: &str = "pigweb_user";

/// The default maximum number of responses a fetch request will return
pub const DEFAULT_API_RESPONSE_LIMIT: u32 = 100;

/// This type is used as a type-level key for [rocket_oauth2] and as the
/// cookie containing the token data.
#[cfg(feature = "server")]
pub struct OpenIDAuth;

/// Attempts to parse a `&str` to a [`uuid::Uuid`], erroring with HTTP status 400
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

/// Attempts to parse a [`&Vec<String>`] to a [`Vec<uuid::Uuid>`], erroring with
/// HTTP status 400
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

/// INTERNAL/COMMON MODULE USE ONLY - generates builder functions for a list of
/// values which can be parsed to a [`String`] (usually [`uuid::Uuid`]s), meant
/// for use when building structs for querying data.
///
/// Example:
/// ```rust
/// use pigweb_common::query_list;
/// use uuid::Uuid;
///
/// pub struct FetchQuery {
///     pub id: Option<Vec<String>>
/// }
///
/// impl FetchQuery {
///     query_list!(id, Uuid);
/// }
/// ```
#[macro_export]
macro_rules! query_list {
    ($var:ident, $input:ty) => {
        // https://users.rust-lang.org/t/can-i-build-a-function-name-from-arguments-to-a-macro-rules/45061/4
        paste::item! {
            pub fn [< with_ $var >] (self, $var: &$input) -> Self {
                self.[< with_ $var s >](&vec![$var.to_owned()])
            }

            pub fn [< with_ $var _string >](self, $var: &String) -> Self {
                self.[< with_ $var s_string >](vec![$var.to_owned()])
            }

            pub fn [< with_ $var s >] (self, $var: &Vec<$input>) -> Self {
                self.[< with_ $var s_string >]($var.iter().map(|e| e.to_string()).collect())
            }

            pub fn [< with_ $var s_string >] (mut self, $var: Vec<String>) -> Self {
                self.$var = Some($var);
                self
            }
        }
    };
}

/// INTERNAL/COMMON MODULE USE ONLY - generates builder functions for setting
/// the limit and offset of a query as [`u32`], meant for use when building
/// structs for querying data.
///
/// Example:
/// ```rust
/// use pigweb_common::query_limit_offset;
///
/// pub struct FetchQuery {
///     pub limit: Option<u32>,
///     pub offset: Option<u32>
/// }
///
/// impl FetchQuery {
///     query_limit_offset!();
/// }
/// ```
#[macro_export]
macro_rules! query_limit_offset {
    () => {
        /// Sets the maximum number of items to return
        pub fn with_limit(mut self, limit: u32) -> Self {
            self.limit = Some(limit);
            self
        }

        /// If the number of items which meet the query params exceeds the
        /// limit, start counting from here
        pub fn with_offset(mut self, offset: u32) -> Self {
            self.offset = Some(offset);
            self
        }
    };
}

/// INTERNAL/COMMON MODULE USE ONLY - generates a function for serializing the
/// struct into a URL at the given root path + `"fetch"` + the query params,
/// meant for use when building structs for querying data. URL is generated with
/// [`yuri`] and [`query`].
///
/// Example:
/// ```rust
/// use pigweb_common::query_to_yuri;
///
/// #[derive(Debug, PartialEq, serde::Serialize)]
/// #[cfg_attr(feature = "server", derive(rocket::FromForm))]
/// pub struct FetchQuery {
///     // data goes here
/// }
///
/// impl FetchQuery {
///     query_to_yuri!("/api/data/");
/// }
/// ```
#[macro_export]
macro_rules! query_to_yuri {
    ($segment:expr) => {
        pub fn to_yuri(&self) -> String {
            $crate::yuri!($segment, "fetch" ;? $crate::query!(self))
        }
    }
}
