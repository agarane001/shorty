use std::collections::HashMap;

use axum::{Form, Json, extract::{Path, Query, State}, http::{self, StatusCode}, response::{IntoResponse, Redirect}};
use tracing::instrument;
use uuid::Uuid;

use crate::{errors::AuthError, routes::auth::Claims, startup::AppState}; // Your JWT Claims struct

use tracing::{info, warn, error};
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct CreateUrlForm{
    pub url: String,
    pub site_name: String
}

#[instrument(name = "Web: Create URL", skip(state, claims, form))]
pub async fn shorten_form_handler(
    State(state): State<AppState>,
    claims: Claims,
    Form(form): Form<CreateUrlForm>,
) -> Result<impl IntoResponse, AuthError> {
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| AuthError::InvalidToken)?;

    // Use your existing service logic
    state.url_service
        .shorten(&form.url,&form.site_name, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to shorten URL: {:?}", e);
            AuthError::Internal
        })?;


    // Redirect back to the dashboard to show the new link in the list
    Ok(Redirect::to("/dashboard"))
}

#[instrument(
    name = "HTTP: Shorten request", 
    skip(state, claims, params), 
    fields(user_id = %claims.sub)
)]
pub async fn shorten(
    State(state): State<AppState>,
    claims: Claims, // Extractor ensures user is authorized
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let url = match params.get("url") {
        Some(url) => url,
        None => return (http::StatusCode::BAD_REQUEST, "Missing url parameter").into_response(),
    };

    let site_name = match params.get("site_name") {
        Some(site_name) => site_name,
        None => return (http::StatusCode::BAD_REQUEST, "Missing site_name parameter").into_response(),
    };

    // Convert the string 'sub' from JWT back to a Uuid
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return (http::StatusCode::UNAUTHORIZED, "Invalid user ID in token").into_response(),
    };

    match state.url_service.shorten(url,site_name,user_id).await {
        Ok(shortened) => {
            Json(json!({ "short_url": shortened })).into_response()
        }
        Err(e) => {
            error!("Failed to shorten URL: {:?}", e);
            (http::StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
        }
    }
}

#[instrument(name = "HTTP: Redirect request", skip(state))]
pub async fn redirect(
    Path(short_url): Path<String>, 
    State(state): State<AppState>
) -> impl IntoResponse {
    if let Some(url) = state.url_service.resolve(&short_url).await {
        info!(short_code = %short_url, "Redirecting to {}", url);
        return Redirect::permanent(url.as_str()).into_response();
    }

    warn!(short_code = %short_url, "Short URL not found");
    (StatusCode::BAD_REQUEST, "url not found").into_response()
}
