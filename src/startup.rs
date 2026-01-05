pub use crate::configuration;
use crate::routes::auth::login_page;
use crate::routes::auth::login_post;
use crate::routes::auth::signup_page;
use crate::routes::auth::signup_post;
use crate::routes::dashboard::dashboard_handler;
use crate::services::auth::AuthService;
use crate::services::url::UrlService;
use crate::store::CacheRepository;
use crate::store::UrlRepository;
use crate::store::user::UserRepository;
use tower_http::services::ServeDir;

use axum::{
    Router,
    routing::{get, post},
};
use redis::AsyncCommands;
use redis::Client;
use sqlx::postgres::PgPoolOptions;

use crate::routes::auth::{authorize_handler, register_handler};
use crate::{
    configuration::get_configuration,
    routes::url::{redirect, shorten},
};

#[derive(Clone, Debug)]
pub struct AppState {
    pub url_service: UrlService,
    pub auth_service: AuthService,
}

pub async fn run() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let cfg = get_configuration().expect("could not get config");

    let client = Client::open(redis_url).expect("could not open a client connection to redis");
    let redis_pool = bb8::Pool::builder().build(client).await.unwrap();
    {
        // ping the database before starting
        let mut conn = redis_pool.get().await.unwrap();
        conn.set::<&str, &str, ()>("foo", "bar").await.unwrap();
        let result: String = conn.get("foo").await.unwrap();
        assert_eq!(result, "bar");
    }

    let pg_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(cfg.database.with_db());

    let repo = UrlRepository::new(pg_pool.clone());
    let cache = CacheRepository::new(redis_pool);
    let url_service = UrlService::new(repo, cache);

    let user_repo = UserRepository::new(pg_pool.clone());
    let auth_service = AuthService::new(user_repo);
    let app_state = AppState {
        url_service,
        auth_service,
    };
    let app = Router::new()
        .route("/dashboard", get(dashboard_handler))
        .route("/url/shorten", get(shorten))
        .route("/url/{key}", get(redirect))
        .route("/register", post(register_handler))
        .route("/login", get(login_page).post(login_post))
        .route("/signup", get(signup_page).post(signup_post))
        .route("/authorize", post(authorize_handler))
        .nest_service(
            "/assets",
            ServeDir::new(format!(
                "{}/public",
                std::env::current_dir().unwrap().to_str().unwrap()
            )),
        )
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:4001")
        .await
        .unwrap();
    axum::serve(listener, app)
        .await
        .expect("could not start server");
}
pub type ConnectionPool = bb8::Pool<Client>;
