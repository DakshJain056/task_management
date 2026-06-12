use axum::{
    extract::{State, Json},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{generate_2fa_code, generate_jwt, hash_password, hash_verification_code, verify_password},
    error::{AppError, AppResult},
    models::{
        AssignTaskRequest, Claims, CreateTaskRequest, LoginRequest, LoginResponse,
        Task, TaskViewResponse, User, Verify2FaRequest, Verify2FaResponse,
    },
    AppState, LatestEmailLog,
};

// --- POST /seed/users ---
// Create Admin and James Bond users for validation.
pub async fn seed_users(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    // We clean up existing seeded users first to make the operation idempotent and clean
    let emails = vec!["admin@example.com", "james.bond@example.com"];
    for email in &emails {
        sqlx::query("DELETE FROM users WHERE email = $1")
            .bind(email)
            .execute(&state.db_pool)
            .await?;
    }

    // 1. Seed Admin user
    let admin_id = Uuid::new_v4();
    let admin_email = "admin@example.com";
    let admin_password_hash = hash_password("admin123")
        .map_err(|e| AppError::Internal(format!("Password hashing failed: {}", e)))?;
    let admin_role = "admin";

    sqlx::query(
        "INSERT INTO users (id, full_name, email, hashed_password, role) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(admin_id)
    .bind("Administrator")
    .bind(admin_email)
    .bind(&admin_password_hash)
    .bind(admin_role)
    .execute(&state.db_pool)
    .await?;

    // 2. Seed James Bond user
    let bond_id = Uuid::new_v4();
    let bond_email = "james.bond@example.com";
    let bond_password_hash = hash_password("bond123")
        .map_err(|e| AppError::Internal(format!("Password hashing failed: {}", e)))?;
    let bond_role = "user";

    sqlx::query(
        "INSERT INTO users (id, full_name, email, hashed_password, role) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(bond_id)
    .bind("James Bond")
    .bind(bond_email)
    .bind(&bond_password_hash)
    .bind(bond_role)
    .execute(&state.db_pool)
    .await?;

    tracing::info!("Users seeded: admin@example.com and james.bond@example.com");

    Ok((
        StatusCode::CREATED,
        Json(json!({ "message": "Users seeded successfully" })),
    ))
}

// --- POST /auth/login ---
// Validate email/password, create a 2FA challenge, and trigger an email code.
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    // Find the user by email
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&state.db_pool)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    // Verify password
    if !verify_password(&payload.password, &user.hashed_password) {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // Generate 6-digit code
    let code = generate_2fa_code();
    let hashed_code = hash_verification_code(&code);

    // Create 2FA challenge (valid for 5 minutes)
    let challenge_id = Uuid::new_v4();
    let expires_at = Utc::now() + chrono::Duration::minutes(5);

    sqlx::query(
        "INSERT INTO login_challenges (id, user_id, code_hash, expires_at, used) VALUES ($1, $2, $3, $4, FALSE)"
    )
    .bind(challenge_id)
    .bind(user.id)
    .bind(&hashed_code)
    .bind(expires_at)
    .execute(&state.db_pool)
    .await?;

    // Trigger an email log
    let subject = "Your 2FA Verification Code";
    let body = format!("Your verification code is: {}", code);

    sqlx::query(
        "INSERT INTO email_logs (user_email, subject, body) VALUES ($1, $2, $3)"
    )
    .bind(&user.email)
    .bind(subject)
    .bind(&body)
    .execute(&state.db_pool)
    .await?;

    // Update in-memory dev email log
    {
        let mut log_guard = state.latest_email_log.lock().await;
        *log_guard = Some(LatestEmailLog {
            email: user.email.clone(),
            verification_code: code.clone(),
            subject: subject.to_string(),
            body: body.clone(),
        });
    }

    tracing::info!("Generated 2FA challenge {} for {} (code: {})", challenge_id, user.email, code);

    Ok((
        StatusCode::OK,
        Json(LoginResponse {
            login_challenge_id: challenge_id,
            message: "2FA code sent to your email".to_string(),
        }),
    ))
}

// --- POST /auth/verify-2fa ---
// Verify the code and return a JWT access token.
pub async fn verify_2fa(
    State(state): State<AppState>,
    Json(payload): Json<Verify2FaRequest>,
) -> AppResult<impl IntoResponse> {
    // Retrieve challenge
    let challenge = sqlx::query_as::<_, crate::models::LoginChallenge>(
        "SELECT * FROM login_challenges WHERE id = $1"
    )
    .bind(payload.login_challenge_id)
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid or expired challenge".to_string()))?;

    // Check if used
    if challenge.used {
        return Err(AppError::Unauthorized("Verification code already used".to_string()));
    }

    // Check if expired
    if challenge.expires_at < Utc::now() {
        return Err(AppError::Unauthorized("Verification code expired".to_string()));
    }

    // Hash user's code and compare
    let input_hash = hash_verification_code(&payload.code);
    if challenge.code_hash != input_hash {
        return Err(AppError::Unauthorized("Incorrect verification code".to_string()));
    }

    // Mark challenge as used
    sqlx::query("UPDATE login_challenges SET used = TRUE WHERE id = $1")
        .bind(challenge.id)
        .execute(&state.db_pool)
        .await?;

    // Get the user
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(challenge.user_id)
        .fetch_one(&state.db_pool)
        .await?;

    // Generate JWT access token
    let token = generate_jwt(user.id, &user.email, &user.role, &state.jwt_secret)
        .map_err(|e| AppError::Internal(format!("JWT generation failed: {}", e)))?;

    tracing::info!("User {} successfully verified 2FA, JWT issued", user.email);

    Ok((
        StatusCode::OK,
        Json(Verify2FaResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
        }),
    ))
}

// --- POST /tasks ---
// Create a task. Admin only.
pub async fn create_task(
    State(state): State<AppState>,
    claims: Claims,
    Json(payload): Json<CreateTaskRequest>,
) -> AppResult<impl IntoResponse> {
    // Authorization check
    if claims.role != "admin" {
        return Err(AppError::Forbidden("Only admins can create tasks".to_string()));
    }

    // Validation
    if payload.title.trim().is_empty() {
        return Err(AppError::BadRequest("Title cannot be empty".to_string()));
    }

    let priority = payload.priority.as_deref().unwrap_or("medium");
    if !matches!(priority, "low" | "medium" | "high") {
        return Err(AppError::BadRequest("Priority must be one of: low, medium, high".to_string()));
    }

    let status = payload.status.as_deref().unwrap_or("todo");
    if !matches!(status, "todo" | "in_progress" | "done") {
        return Err(AppError::BadRequest("Status must be one of: todo, in_progress, done".to_string()));
    }

    // Check if assignee exists (if provided)
    if let Some(assignee_id) = payload.assigned_to_id {
        let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
            .bind(assignee_id)
            .fetch_one(&state.db_pool)
            .await?;
        if !exists {
            return Err(AppError::BadRequest("Assigned user does not exist".to_string()));
        }
    }

    // Insert task
    let task = sqlx::query_as::<_, Task>(
        "INSERT INTO tasks (title, description, status, priority, created_by_id, assigned_to_id) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"
    )
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(status)
    .bind(priority)
    .bind(claims.sub)
    .bind(payload.assigned_to_id)
    .fetch_one(&state.db_pool)
    .await?;

    // Invalidate the cache completely to ensure consistency
    state.cache.clear().await;
    tracing::info!("Task '{}' created by admin, cache cleared", task.title);

    Ok((StatusCode::CREATED, Json(task)))
}

// --- POST /tasks/assign ---
// Assign selected tasks to James Bond (or the specified user). Admin only.
pub async fn assign_tasks(
    State(state): State<AppState>,
    claims: Claims,
    Json(payload): Json<AssignTaskRequest>,
) -> AppResult<impl IntoResponse> {
    // Authorization check
    if claims.role != "admin" {
        return Err(AppError::Forbidden("Only admins can assign tasks".to_string()));
    }

    // Validation
    if payload.task_ids.is_empty() {
        return Err(AppError::BadRequest("No task IDs provided".to_string()));
    }

    // Find the assignee
    let assignee_email = payload
        .user_email
        .as_deref()
        .unwrap_or("james.bond@example.com");

    let assignee = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(assignee_email)
        .fetch_optional(&state.db_pool)
        .await?
        .ok_or_else(|| AppError::BadRequest(format!("Assignee user '{}' not found", assignee_email)))?;

    // Update tasks
    let result = sqlx::query(
        "UPDATE tasks SET assigned_to_id = $1, updated_at = NOW() WHERE id = ANY($2)"
    )
    .bind(assignee.id)
    .bind(&payload.task_ids)
    .execute(&state.db_pool)
    .await?;

    let rows_affected = result.rows_affected();
    tracing::info!("Assigned {} tasks to {}, rows affected: {}", payload.task_ids.len(), assignee_email, rows_affected);

    // Invalidate the cache completely to ensure consistency
    state.cache.clear().await;

    Ok((
        StatusCode::OK,
        Json(json!({
            "message": format!("Successfully assigned tasks to {}", assignee_email),
            "tasks_assigned_count": rows_affected
        })),
    ))
}

// --- GET /tasks/view-my-tasks ---
// Return tasks assigned to the logged-in user, with cache metadata.
pub async fn view_my_tasks(
    State(state): State<AppState>,
    claims: Claims,
) -> AppResult<impl IntoResponse> {
    // 1. Check cache
    if let Some(mut cached_response) = state.cache.get(&claims.sub).await {
        let mut headers = HeaderMap::new();
        headers.insert("X-Cache", "HIT".parse().unwrap());
        
        // Ensure user details are up-to-date in response
        cached_response.user = crate::models::UserInfo {
            email: claims.email.clone(),
            role: claims.role.clone(),
        };
        
        return Ok((StatusCode::OK, headers, Json(cached_response)));
    }

    // 2. Cache Miss: Query database
    let tasks = if claims.role == "admin" {
        // Admin sees all tasks
        sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks ORDER BY created_at DESC"
        )
        .fetch_all(&state.db_pool)
        .await?
    } else {
        // Regular user sees only their assigned tasks
        sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks WHERE assigned_to_id = $1 ORDER BY created_at DESC"
        )
        .bind(claims.sub)
        .fetch_all(&state.db_pool)
        .await?
    };

    let user_info = crate::models::UserInfo {
        email: claims.email.clone(),
        role: claims.role.clone(),
    };

    let response = TaskViewResponse {
        tasks,
        cache_hit: false,
        user: user_info,
    };

    // 3. Set cache
    state.cache.set(claims.sub, response.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert("X-Cache", "MISS".parse().unwrap());

    Ok((StatusCode::OK, headers, Json(response)))
}

// --- GET /dev/email-logs/latest ---
// Development-only endpoint to view the latest sent verification code.
pub async fn latest_email_log(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    let log_guard = state.latest_email_log.lock().await;
    match &*log_guard {
        Some(log) => Ok((StatusCode::OK, Json(log.clone()))),
        None => Err(AppError::NotFound("No verification email sent yet".to_string())),
    }
}
