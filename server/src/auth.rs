use crate::config::Config;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use pigweb_common::users::{User, SYSTEM_USER};
use pigweb_common::{users, OpenIDAuth, COOKIE_JWT, COOKIE_USER};
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::outcome::try_outcome;
use rocket::outcome::Outcome::{Error, Forward, Success};
use rocket::request::{FromRequest, Outcome};
use rocket::response::Redirect;
use rocket::serde::json::{serde_json, Value};
use rocket::{Request, Route};
use rocket_oauth2::{OAuth2, TokenResponse};
use std::collections::BTreeMap;

pub struct AuthenticatedUser {
    pub jwt: Option<Value>,
    pub user: User,
}

impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = &'static str;

    // see https://github.com/jebrosen/rocket_oauth2/blob/b0971d6d6e0e1422306e397bc3e018c1ec822013/examples/user_info/src/main.rs#L18-L30
    async fn from_request(request: &'r Request<'_>) -> Outcome<AuthenticatedUser, Self::Error> {
        // First, check the config to see if authentication is actually configured
        let config = try_outcome!(request.guard::<&Config>().await);

        // If authentication isn't configured, pass the challenge and return the system user
        if config.oidc.as_ref().is_none() {
            return Success(AuthenticatedUser { jwt: None, user: SYSTEM_USER });
        }

        // If there are any errors fetching the cookies, pass it on
        let cookies = try_outcome!(request.guard::<&CookieJar<'_>>().await);

        // Get the JWT cookie and attempt to parse it to a Value
        if let Some(cookie) = cookies.get_private(COOKIE_JWT) {
            if let Some(jwt) = serde_json::from_str(cookie.value()).ok() {

                // We're only allowed to use the subject (sub) and issuer (iss) from OIDC to uniquely identify a user
                // https://openid.net/specs/openid-connect-core-1_0.html#ClaimStability

                // TODO check expiration on JWT, invalidate session if it has expired
                // TODO fetch user info from db for valid JWT, make sure DB user info is up to date, then return that as a Success
                //return Success(GenericOAuth { jwt });
            }
        }

        // If there are any errors, you're probably unauthorized
        Forward(Status::Unauthorized)
    }
}

pub fn get_auth_api_routes() -> Vec<Route> {
    routes![oidc_login, oidc_auth, oidc_logout]
}

#[get("/")]
async fn is_authenticated(_user: &AuthenticatedUser) -> Status {
    // If the user isn't signed in, it should return a 401 unauthorized
    // TODO CHECK THIS WORKS AS INTENDED
    Status::Ok
}

// Redirects users to the login page
#[get("/login/oidc")]
async fn oidc_login(oauth2: OAuth2<OpenIDAuth>, config: &Config, cookies: &CookieJar) -> Redirect {
    // Only force the user to login if it's actually configured
    if let Some(oidc_config) = config.oidc.as_ref() {
        // Convert Vec<String> into &[&str]
        let scopes_slice = oidc_config.scopes.iter().map(|e| e.as_str()).collect::<Vec<&str>>().as_slice();
        return oauth2.get_redirect(cookies, scopes_slice).unwrap();
    }

    Redirect::to("/")
}

// Completes the token exchange with the OAuth provider, creates a session
// cookie, then redirects the user to the app root
#[get("/auth/oidc")]
async fn oidc_auth(
    token_response: TokenResponse<OpenIDAuth>,
    config: &Config,
    cookies: &CookieJar,
) -> Result<Redirect, Status> {
    // Only force the user to login if it's actually configured
    if config.oidc.as_ref().is_none() {
        return Ok(Redirect::to("/"));
    }

    // Get the OIDC config and response JSON values
    // What the token response should look like: https://openid.net/specs/openid-connect-core-1_0.html#TokenResponse
    let oidc_config = config.oidc.as_ref().unwrap();
    let response_values = token_response.as_value();

    // yes, this is nested quite a bit, but autism has forced my hand in covering error cases
    // first, make sure we have an oidc id_token
    if let Some(id_token_val) = response_values.get("id_token") {
        // then, make sure the id_token is actually a string
        if let Some(id_token) = id_token_val.as_str() {
            // after that, decode the JWT and verify the signature
            let decode_result = jsonwebtoken::decode::<BTreeMap<String, String>>(
                &id_token,
                &DecodingKey::from_secret(oidc_config.client_secret.as_ref()),
                &Validation::new(Algorithm::RS256),
            );

            if let Ok(jwt) = decode_result {
                // Finally, convert the JWT claims back into a JSON string and set the cookie for it
                cookies.add_private(
                    Cookie::build((
                        COOKIE_JWT,
                        serde_json::to_string(&jwt.claims).unwrap_or_else(|e| {
                            error!("Unable to convert JWT back into JSON: {:?}", e);
                            "{}".to_owned()
                        }),
                    ))
                    .same_site(SameSite::Lax)
                    .build(),
                );

                // FINALLY return our OK case
                return Ok(Redirect::to("/"));
            } else if let Err(e) = decode_result {
                error!("Unable to parse or validate JWT: {:?}", e);
            }
        } else {
            error!("Unable to convert the id_token to a &str: {:?}", id_token_val.to_string());
        }
    } else {
        error!("Unable to find id_token on OIDC response: {:?}", response_values.as_str());
    }

    // Putting this at the end so we don't have to duplicate the return statement
    Err(Status::InternalServerError)
}

// Removes the user's current session cookies and redirects them to the OIDC
// provider logout page (if present) or the root page
#[get("/logout/oidc")]
async fn oidc_logout(config: &Config, cookies: &CookieJar) -> Redirect {
    // Remove the current JWT and USER cookies
    cookies.remove_private(COOKIE_JWT);
    cookies.remove_private(COOKIE_USER);

    // Redirect the user to the OIDC provider logout page, if present
    if let Some(oidc_config) = config.oidc.as_ref() {
        if let Some(logout_uri) = oidc_config.logout_uri.as_ref() {
            return Redirect::to(logout_uri);
        }
    }

    // Redirect the user to root as a last resort
    Redirect::to("/")
}
