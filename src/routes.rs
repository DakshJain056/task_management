use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    handlers::{assign_tasks, create_task, latest_email_log, login, seed_users, verify_2fa, view_my_tasks},
    AppState,
};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Seeding endpoint
        .route("/seed/users", post(seed_users))
        // Authentication endpoints
        .route("/auth/login", post(login))
        .route("/auth/verify-2fa", post(verify_2fa))
        // Development support endpoints
        .route("/dev/email-logs/latest", get(latest_email_log))
        // Task management endpoints
        .route("/tasks", post(create_task))
        .route("/tasks/assign", post(assign_tasks))
        .route("/tasks/view-my-tasks", get(view_my_tasks))
        // Shared application state
        .with_state(state)
}
