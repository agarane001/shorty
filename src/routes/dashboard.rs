use crate::{models::url::UrlModel, routes::auth::Claims, startup::AppState};
use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    email: String,
    urls: Vec<UrlModel>,
}

pub async fn dashboard_handler(
    State(state): State<AppState>,
    claims: Claims, // Authenticated user
) -> impl IntoResponse {
    // 1. Fetch user URLs from DB
    let user_id = uuid::Uuid::parse_str(&claims.sub).unwrap();
    let urls = state
        .url_service
        .get_user_urls(user_id)
        .await
        .unwrap_or_default();

    // 2. Render Template
    let template = DashboardTemplate {
        email: claims.sub, // Or fetch email from DB
        urls,
    };
    Html(template.render().unwrap())
}
