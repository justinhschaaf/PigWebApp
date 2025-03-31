pub mod pigs;
pub mod users;
pub mod yuri;

#[cfg(feature = "server")]
pub mod schema;

/// The relative base URL for all authentication API routes
pub const AUTH_API_ROOT: &str = "/auth/";

/// The relative base URL for all Pig API routes
pub const PIG_API_ROOT: &str = "/api/pigs/";

#[cfg(feature = "server")]
pub const COOKIE_JWT: &str = "pigweb_jwt";
pub const COOKIE_USER: &str = "pigweb_user";

/// This type is used as a type-level key for rocket_oauth2 and as the
/// cookie containing the token data.
#[cfg(feature = "server")]
pub struct OpenIDAuth;
