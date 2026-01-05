use std::collections::HashMap;

use axum::{Json, extract::{Path, Query, State}, http::{self, StatusCode}, response::{IntoResponse, Redirect}};
use tracing::instrument;
use uuid::Uuid;

use crate::{routes::auth::Claims, startup::AppState}; // Your JWT Claims struct

use tracing::{info, warn, error};
use serde_json::json;

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

    // Convert the string 'sub' from JWT back to a Uuid
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return (http::StatusCode::UNAUTHORIZED, "Invalid user ID in token").into_response(),
    };

    match state.url_service.shorten(url, user_id).await {
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