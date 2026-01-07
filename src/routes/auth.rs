use axum_extra::extract::CookieJar;
use std::fmt::Display;
use std::sync::LazyLock;

use askama::Template;
use axum::Form;
use axum::Json;
use axum::RequestPartsExt;
use axum::extract::{FromRequestParts, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum_extra::extract::TypedHeader;
use axum_extra::extract::cookie::Cookie;
use axum_extra::headers::{Authorization, authorization::Bearer};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::errors::AuthError;
use crate::startup::AppState;

#[derive(Template)]
#[template(path = "signup.html")]
struct SignupTemplate {
    email: String,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    email: String,
}

pub async fn signup_page() -> impl IntoResponse {
    Html(SignupTemplate { email: "".into() }.render().unwrap())
}

pub async fn login_page() -> impl IntoResponse {
    Html(LoginTemplate { email: "".into() }.render().unwrap())
}

#[instrument(name = "Web: Login POST", skip(state, jar, payload))]
pub async fn login_post(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(payload): Form<AuthPayload>,
) -> Result<impl IntoResponse, AuthError> {
    tracing::info!("Request to login user recieved!");
    // 1. Verify credentials via service
    let user_id = state
        .auth_service
        .login(&payload.email, &payload.password)
        .await?;

    // 2. Create JWT
    let claims = Claims {
        sub: user_id.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
    };
    let token = encode(&Header::default(), &claims, &KEYS.encoding)
        .map_err(|_| AuthError::TokenCreation)?;

    // 3. Set HttpOnly Cookie and Redirect to Dashboard
    let cookie = Cookie::build(("jwt", token))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax);

    Ok((jar.add(cookie), Redirect::to("/dashboard")))
}

#[instrument(name = "Web: Signup POST", skip(state, payload))]
pub async fn signup_post(
    State(state): State<AppState>,
    Form(payload): Form<AuthPayload>,
) -> Result<impl IntoResponse, AuthError> {
    state
        .auth_service
        .register(&payload.email, &payload.password)
        .await
        .map_err(|_| AuthError::Internal)?;

    Ok(Redirect::to("/login"))
}

#[instrument(name = "Web: Logout GET", skip(jar))]
pub async fn logout_handler(jar: CookieJar) -> impl IntoResponse {
    let updated_jar = jar.remove(Cookie::from("jwt"));
    (updated_jar, Redirect::to("/login"))
}

static KEYS: LazyLock<Keys> = LazyLock::new(|| {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    Keys::new(secret.as_bytes())
});

pub struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

#[derive(Debug, Serialize)]
pub struct AuthBody {
    access_token: String,
    token_type: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthPayload {
    email: String,
    password: String,
}

impl Display for Claims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Email: {}", self.sub)
    }
}

impl AuthBody {
    fn new(access_token: String) -> Self {
        Self {
            access_token,
            token_type: "Bearer".to_string(),
        }
    }
}

#[instrument(name = "register new user", skip(state))]
pub async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<AuthPayload>,
) -> Result<impl IntoResponse, AuthError> {
    state
        .auth_service
        .register(&payload.email, &payload.password)
        .await
        .map_err(|_| AuthError::UserAlreadyExists)?;

    Ok(StatusCode::CREATED)
}

#[instrument(
    name = "HTTP: Authorize Handler",
    skip(state, payload),
    fields(
        user_email = %payload.email,
        request_id = tracing::field::Empty
    )
)]
pub async fn authorize_handler(
    State(state): State<AppState>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthBody>, AuthError> {
    tracing::info!("Received login request");

    let user_id = state
        .auth_service
        .login(&payload.email, &payload.password)
        .await
        .map_err(|e| {
            tracing::error!("Authorization failed: {:?}", e);
            e
        })?;

    let claims = Claims {
        sub: user_id.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
    };

    let token = encode(&Header::default(), &claims, &KEYS.encoding).map_err(|e| {
        tracing::error!("JWT Encoding failed: {:?}", e);
        AuthError::TokenCreation
    })?;

    tracing::info!("JWT issued for user");
    Ok(Json(AuthBody::new(token)))
}

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    #[instrument(name = "Extracting Claims", skip(_state, parts))]
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Try to get token from Cookies (for Browser/Dashboard)
        let cookie_token = parts
            .extract::<CookieJar>()
            .await
            .ok()
            .and_then(|jar| jar.get("jwt").map(|c| c.value().to_string()));

        // 2. If no cookie, try to get from Authorization Header (for API/Curl)
        let token = if let Some(t) = cookie_token {
            t
        } else {
            let TypedHeader(Authorization(bearer)) = parts
                .extract::<TypedHeader<Authorization<Bearer>>>()
                .await
                .map_err(|_| {
                    tracing::warn!("No JWT found in cookies or headers");
                    AuthError::InvalidToken
                })?;
            bearer.token().to_string()
        };

        // 3. Decode the token
        let token_data =
            decode::<Claims>(&token, &KEYS.decoding, &Validation::default()).map_err(|e| {
                tracing::error!("JWT decoding failed: {:?}", e);
                AuthError::InvalidToken
            })?;

        Ok(token_data.claims)
    }
}
