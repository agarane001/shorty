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
    total_clicks: i32,
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
    let total_clicks = urls.iter().map(|u| u.clicks).sum();

    // 2. Render Template
    let template = DashboardTemplate {
        email: claims.sub,
        urls,
        total_clicks,
    };
    Html(template.render().unwrap())
}
