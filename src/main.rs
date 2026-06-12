mod auth;
mod cache;
mod error;
mod handlers;
mod models;
mod routes;

use std::sync::Arc;
use tokio::sync::Mutex;

use cache::TaskCache;
use routes::create_router;

#[derive(Debug, Clone, serde::Serialize)]
pub struct LatestEmailLog {
    pub email: String,
    pub verification_code: String,
    pub subject: String,
    pub body: String,
}

#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub jwt_secret: String,
    pub cache: TaskCache,
    pub latest_email_log: Arc<Mutex<Option<LatestEmailLog>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize logging / tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "task_management=info,tower_http=info,axum=info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/task_management".to_string());

    tracing::info!("Connecting to PostgreSQL database...");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;
    tracing::info!("Database migrations run successfully.");

    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    tracing::info!("Connecting to Redis cache at {}...", redis_url);
    let cache = TaskCache::new(&redis_url).await.map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to connect to Redis: {}", e))
    })?;

    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "super_secret_jwt_key_that_is_long_enough_to_be_secure_123456".to_string());

    let state = AppState {
        db_pool: pool,
        jwt_secret,
        cache,
        latest_email_log: Arc::new(Mutex::new(None)),
    };

    let app = create_router(state);

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("HTTP server starting on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
