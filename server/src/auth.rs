use crate::config::Config;
use crate::userapi::{get_user_roles, user_has_role};
use chrono::{DateTime, Utc};
use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
use diesel::{
    ExpressionMethods, NullableExpressionMethods, PgConnection, QueryDsl, QueryResult, RunQueryDsl, SelectableHelper,
};
use jsonwebtoken::{DecodingKey, Validation};
use pigweb_common::users::{Roles, User};
use pigweb_common::{schema, OpenIDAuth, COOKIE_JWT, COOKIE_USER};
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::outcome::try_outcome;
use rocket::outcome::Outcome::{Error, Success};
use rocket::request::{FromRequest, Outcome};
use rocket::response::Redirect;
use rocket::serde::json::{serde_json, Json};
use rocket::serde::{Deserialize, Serialize};
use rocket::{Request, Route, State};
use rocket_oauth2::{OAuth2, TokenResponse};
use std::collections::BTreeSet;
use std::ops::DerefMut;
use std::sync::Mutex;

/// A [Request Guard](FromRequest) which requires the user be signed in with an
/// active session before accessing the given route.
pub struct AuthenticatedUser {
    /// The claims retrieved from the JWT provided by the OIDC provider
    pub jwt: Option<Claims>,

    /// The user data backing this AuthenticatedUser
    pub user: User,
}

impl AuthenticatedUser {
    /// Removes the app's session cookies and returns HTTP status code 401
    fn invalidate_session(cookies: &CookieJar) -> Outcome<AuthenticatedUser, ()> {
        cookies.remove_private(COOKIE_JWT);
        cookies.remove_private(COOKIE_USER);
        Error((Status::Unauthorized, ()))
    }

    /// Whether this user is in a group which provides the given Role.
    ///
    /// ***Always returns true if OIDC or groups are not configured.***
    pub fn has_role(&self, config: &Config, role: Roles) -> bool {
        user_has_role(config, &self.user, role)
    }

    /// Gets all roles this user has been provided by their groups.
    ///
    /// ***Returns a set of all roles if the OIDC or groups are not configured.***
    pub fn get_roles(&self, config: &Config) -> BTreeSet<Roles> {
        get_user_roles(config, &self.user)
    }
}

// adding async_trait resolves E0195, somehow... https://stackoverflow.com/a/69271844
#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    // This must be nothing for try_outcome!() to work
    type Error = ();

    // see https://github.com/jebrosen/rocket_oauth2/blob/b0971d6d6e0e1422306e397bc3e018c1ec822013/examples/user_info/src/main.rs#L18-L30
    async fn from_request(request: &'r Request<'_>) -> Outcome<AuthenticatedUser, ()> {
        // Get the request guards we need
        let config = try_outcome!(request.guard::<&State<Config>>().await);
        let cookies = request.cookies();
        let db_connection = try_outcome!(request.guard::<&State<Mutex<PgConnection>>>().await);

        // First, check the config to see if authentication is actually configured
        // If authentication isn't configured, pass the challenge and return the system user
        if config.oidc.as_ref().is_none() {
            return Success(AuthenticatedUser { jwt: None, user: User::get_system_user() });
        }

        // Get the JWT cookie and attempt to parse it to a Value
        if let Some(jwt_cookie) = cookies.get_private(COOKIE_JWT) {
            // We need to declare this first because the if-let statement doesn't like turbofish syntax to determine the inner type
            let jwt_opt: Option<Claims> = serde_json::from_str(jwt_cookie.value()).ok();
            if let Some(jwt) = jwt_opt {
                // Check if the JWT expired, or if we couldn't get it, expire it anyway
                if jwt.exp * 1000 <= Utc::now().timestamp_millis() {
                    // Invalidate the session
                    return AuthenticatedUser::invalidate_session(cookies);
                }

                let mut db_connection = db_connection.lock().unwrap();
                let mut user_res: Option<User> = None;

                // If we already have a user cookie
                if let Some(user_cookie) = cookies.get_private(COOKIE_USER) {
                    user_res = serde_json::from_str(user_cookie.value()).ok();

                    // If we can't read the user cookie, assume it's invalid
                    if user_res.is_none() {
                        return AuthenticatedUser::invalidate_session(cookies);
                    }

                    // At this point, we only need to check if the user is expired in the DB
                    let sql_res = schema::users::table
                        .filter(schema::users::columns::id.eq(user_res.as_ref().unwrap().id))
                        .limit(1)
                        .select(schema::users::columns::session_exp.nullable())
                        .load::<Option<NaiveDateTime>>(db_connection.deref_mut());

                    // We don't care about the error condition here
                    if let Ok(res) = sql_res {
                        if res.len() > 0 {
                            if let Some(db_exp) = res[0] {
                                // If the expiration as per the db has passed, invalidate the session
                                if db_exp.to_owned() <= Utc::now().naive_utc() {
                                    return AuthenticatedUser::invalidate_session(cookies);
                                }
                            }
                        }
                    }
                } else {
                    // Get the user info from the DB. We're only allowed to use
                    // the subject (sub) and issuer (iss) from OIDC to uniquely
                    // identify a user.
                    // https://openid.net/specs/openid-connect-core-1_0.html#ClaimStability
                    let jwt_issuer = jwt.iss.to_owned();
                    let jwt_subject = jwt.sub.to_owned();

                    // Fetch the user from the DB by the jwt_issuer and jwt_subject
                    let user_result: QueryResult<Vec<User>> = schema::users::table
                        .filter(schema::users::columns::sso_issuer.eq(jwt_issuer))
                        .filter(schema::users::columns::sso_subject.eq(jwt_subject))
                        .limit(1) // There should only be 1 user with this issuer and subject
                        .select(User::as_select())
                        .load(db_connection.deref_mut());

                    // Whether we need to create a new user
                    let mut create_new_user = true;

                    // If we have a user response
                    if let Ok(user_vec) = user_result {
                        if user_vec.len() > 0 {
                            // DO NOT check session expiration here as if the
                            // user did not have a user cookie saved, then the
                            // session will already be expired in the db.
                            let mut user = user_vec[0].to_owned();

                            // Update our user info from the new JWT info
                            user.seen = Utc::now().naive_utc();
                            user.session_exp =
                                Some(DateTime::from_timestamp(jwt.exp, 0).unwrap_or_default().naive_utc());

                            if let Some(preferred_username) = jwt.preferred_username.as_ref() {
                                user.username = preferred_username.to_owned();
                            }

                            if let Some(groups) = jwt.groups.as_ref() {
                                user.groups = groups.to_owned();
                            }

                            // Put the new user info on our DB
                            let sql_res = diesel::update(schema::users::table)
                                .set(&user)
                                .get_result::<User>(db_connection.deref_mut());

                            if sql_res.is_ok() {
                                // Save the user result
                                user_res = Some(user);
                                create_new_user = false;
                            } else {
                                error!("Unable to update user {:?}: {:?}", user, sql_res.unwrap_err());
                                return Error((Status::InternalServerError, ()));
                            }
                        }
                    }

                    // This is first time login, we need to create the user
                    if create_new_user {
                        if let Some(preferred_username) = jwt.preferred_username.as_ref() {
                            // Create a new user
                            let session_exp =
                                DateTime::from_timestamp(jwt.exp.to_owned(), 0).unwrap_or_default().naive_utc();
                            let user = User::new(
                                preferred_username.to_owned(),
                                jwt.groups.as_ref().unwrap_or(&Vec::new()).to_owned(), // &Vec doesn't implement default()
                                jwt.sub.to_owned(),
                                jwt.iss.to_owned(),
                                Some(session_exp),
                            );

                            // ...and save it to the DB
                            let sql_res = diesel::insert_into(schema::users::table)
                                .values(&user)
                                .execute(db_connection.deref_mut());

                            if sql_res.is_ok() {
                                user_res = Some(user);
                            } else {
                                error!("Unable to save new user {:?}: {:?}", user, sql_res.unwrap_err());
                                return Error((Status::InternalServerError, ()));
                            }
                        }
                    }
                }

                // Return the user if we have it
                if user_res.is_some() {
                    // Save the user cookie
                    cookies.add_private(
                        Cookie::build((
                            COOKIE_USER,
                            serde_json::to_string(&user_res).unwrap_or_else(|e| {
                                error!("Unable to convert User struct into JSON: {:?}", e);
                                "{}".to_owned()
                            }),
                        ))
                        .same_site(SameSite::Lax)
                        .build(),
                    );

                    return Success(AuthenticatedUser { jwt: Some(jwt), user: user_res.unwrap() });
                }
            }
        }

        // If there are any errors, you're probably unauthorized
        AuthenticatedUser::invalidate_session(cookies)
    }
}

/// Represents the claims returned by a JWT response. Includes all [mandatory
/// claims](https://openid.net/specs/openid-connect-core-1_0.html#IDToken) as
/// defined in the spec along with the few [optional claims](https://openid.net/specs/openid-connect-core-1_0.html#StandardClaims)
/// we care about. Any other information is discarded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Issuer Identifier for the issuer of the response. Should be a
    /// case-sensitive URL without no query or fragment components.
    pub iss: String,

    /// Subject Identifier. A locally unique and never-reassigned identifier
    /// within the Issuer for the end-uer. Case-sensitive string no longer than
    /// 255 ASCII characters long.
    pub sub: String,

    /// Audience(s) that this ID Token is intended for. Must contain the client
    /// ID.
    pub aud: String,

    /// Expiration time on or after which the ID Token MUST NOT be accepted, in
    /// seconds since Unix Epoch.
    pub exp: i64,

    /// The time at which the JWT was issued, in seconds since Unix Epoch.
    pub iat: i64,

    /// The time at which end-user authentication occurred, in seconds since
    /// Unix Epoch.
    pub auth_time: Option<i64>,

    /// Shorthand name by which the end-user wishes to be referred to.
    pub preferred_username: Option<String>,

    /// List of groups the end-user possesses within the Issuer.
    pub groups: Option<Vec<String>>,
}

/// Returns a list of all auth api routes
pub fn get_auth_api_routes() -> Vec<Route> {
    routes![is_authenticated, oidc_login, oidc_response, oidc_logout]
}

/// Checks whether the user has a valid session.
/// - If the user isn't signed in, returns status 401 unauthorized
/// - If the user is signed in, returns status 200 with a JSON list of all roles
///   the user has.
#[get("/")]
async fn is_authenticated(user: AuthenticatedUser, config: &State<Config>) -> Json<BTreeSet<Roles>> {
    Json(user.get_roles(config))
}

/// Redirects users to the configured OIDC login page
#[get("/oidc/login")]
async fn oidc_login(oauth2: OAuth2<OpenIDAuth>, config: &State<Config>, cookies: &CookieJar<'_>) -> Redirect {
    // Only force the user to login if it's actually configured
    if let Some(oidc_config) = config.oidc.as_ref() {
        // Convert Vec<String> into &[&str], rust complains if scopes_vec isn't saved on its own
        let scopes_vec = oidc_config.scopes.iter().map(|e| e.as_str()).collect::<Vec<&str>>();
        let scopes_slice = scopes_vec.as_slice();
        return oauth2.get_redirect(cookies, scopes_slice).unwrap();
    }

    Redirect::to("/")
}

/// Completes the token exchange with the OAuth provider, creates a session
/// cookie, then redirects the user to the app root.
#[get("/oidc/response")]
async fn oidc_response(
    token_response: TokenResponse<OpenIDAuth>,
    config: &State<Config>,
    cookies: &CookieJar<'_>,
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
            let mut validation = Validation::default();
            validation.insecure_disable_signature_validation(); // skip validating alg param
            validation.set_audience(&[oidc_config.client_id.to_owned()]); // validate aud, should be the client id

            // after that, decode the JWT and verify the signature
            let decode_result = jsonwebtoken::decode::<Claims>(
                &id_token,
                &DecodingKey::from_secret(oidc_config.client_secret.as_ref()),
                &validation,
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

/// Removes the user's current session cookies and redirects them to the OIDC
/// provider logout page (if present) or the root page
#[get("/oidc/logout")]
async fn oidc_logout(config: &State<Config>, cookies: &CookieJar<'_>) -> Redirect {
    // Remove the current JWT and USER cookies
    cookies.remove_private(COOKIE_JWT);
    cookies.remove_private(COOKIE_USER);

    // TODO update session exp in db?

    // Redirect the user to the OIDC provider logout page, if present
    // use `and_then` to bypass having to save `config.oidc.is_some()` separately
    // look at past revisions of this file for more context
    if let Some(logout_uri) = config.oidc.as_ref().and_then(|oidc_config| oidc_config.logout_uri.to_owned()) {
        return Redirect::to(logout_uri);
    }

    // Redirect the user to root as a last resort
    Redirect::to("/")
}
