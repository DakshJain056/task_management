# Task Management Service

A production-grade, asynchronous task management backend built with Rust, Axum, SQLx, Postgres, and Redis. It features a secure login flow with two-factor authentication (2FA), role-based access control, and an optimized caching system using Redis.

---

## 🛠️ Prerequisites & Setup

Ensure the following dependencies are installed and running on your system:
- **Rust** (stable toolchain)
- **PostgreSQL**
- **Redis** (running on port `6379`)
- **Docker** (optional, to run Redis)

### 1. Database & Cache Configuration
Create a `.env` file in the root directory:
```env
DATABASE_URL=postgres://postgres:postgres@localhost/task_management
REDIS_URL=redis://127.0.0.1:6379
JWT_SECRET=super_secret_jwt_key_that_is_long_enough_to_be_secure_123456
RUST_LOG=info
```

### 2. Run Redis in Docker (if not installed locally)
```bash
docker run -d --name redis -p 6379:6379 redis
```

---

## 🚀 Running the Project

1. Run programmatic migrations and start the web server:
   ```bash
   cargo run
   ```
2. The HTTP server will bind to `http://0.0.0.0:8080`.

---

## 💾 Model Structures

### User Model
```rust
pub struct User {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub hashed_password: String,
    pub role: String, // "admin" or "user"
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Task Model
```rust
pub struct Task {
    pub id: i32,
    pub title: String,
    pub description: Option<String>,
    pub status: String,   // "todo", "in_progress", "done"
    pub priority: String, // "low", "medium", "high"
    pub created_by_id: Option<Uuid>,
    pub assigned_to_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

---

## 🚀 API Documentation

| Method | Endpoint | Description | Auth Type | Access |
| :--- | :--- | :--- | :--- | :--- |
| `POST` | `/seed/users` | Seeds default `admin` and `James Bond` accounts. | None | Public |
| `POST` | `/auth/login` | Validates credentials, issues a 2FA challenge. | None | Public |
| `POST` | `/auth/verify-2fa` | Validates 2FA code and issues JWT token. | None | Public |
| `GET` | `/dev/email-logs/latest` | Dev log retrieval for simulated 2FA email code. | None | Development |
| `POST` | `/tasks` | Creates a new task. | Bearer JWT | Admin Only |
| `POST` | `/tasks/assign` | Assigns list of task IDs to a user. | Bearer JWT | Admin Only |
| `GET` | `/tasks/view-my-tasks` | Retrieves user's tasks with Redis cache header. | Bearer JWT | Authenticated |

---

## 📋 Sample API Response: `GET /tasks/view-my-tasks`

### Cache MISS (First Request)
- **Status Code**: `200 OK`
- **Headers**: `X-Cache: MISS`
```json
{
  "tasks": [
    {
      "id": 13,
      "title": "Obtain Aston Martin DB5 gadgets",
      "description": "Meet Q to collect modified vehicle.",
      "status": "done",
      "priority": "medium",
      "created_by_id": "8193154c-76a1-4c86-af2c-657108f45506",
      "assigned_to_id": "45bf9aff-b660-4124-b490-028f12dc5888",
      "created_at": "2026-06-12T07:22:11.618281Z",
      "updated_at": "2026-06-12T07:22:11.630781Z"
    },
    {
      "id": 11,
      "title": "Infiltrate Janus syndicate",
      "description": "Investigate crime organization in St. Petersburg.",
      "status": "in_progress",
      "priority": "high",
      "created_by_id": "8193154c-76a1-4c86-af2c-657108f45506",
      "assigned_to_id": "45bf9aff-b660-4124-b490-028f12dc5888",
      "created_at": "2026-06-12T07:22:11.609211Z",
      "updated_at": "2026-06-12T07:22:11.630781Z"
    },
    {
      "id": 10,
      "title": "Recover GoldenEye control keys",
      "description": "Obtain keys from Severnaya base.",
      "status": "todo",
      "priority": "high",
      "created_by_id": "8193154c-76a1-4c86-af2c-657108f45506",
      "assigned_to_id": "45bf9aff-b660-4124-b490-028f12dc5888",
      "created_at": "2026-06-12T07:22:11.598063Z",
      "updated_at": "2026-06-12T07:22:11.630781Z"
    }
  ],
  "cache_hit": false,
  "user": {
    "email": "james.bond@example.com",
    "role": "user"
  }
}
```

### Cache HIT (Subsequent Requests)
- **Status Code**: `200 OK`
- **Headers**: `X-Cache: HIT`
```json
{
  "tasks": [
    {
      "id": 13,
      "title": "Obtain Aston Martin DB5 gadgets",
      "description": "Meet Q to collect modified vehicle.",
      "status": "done",
      "priority": "medium",
      "created_by_id": "8193154c-76a1-4c86-af2c-657108f45506",
      "assigned_to_id": "45bf9aff-b660-4124-b490-028f12dc5888",
      "created_at": "2026-06-12T07:22:11.618281Z",
      "updated_at": "2026-06-12T07:22:11.630781Z"
    },
    {
      "id": 11,
      "title": "Infiltrate Janus syndicate",
      "description": "Investigate crime organization in St. Petersburg.",
      "status": "in_progress",
      "priority": "high",
      "created_by_id": "8193154c-76a1-4c86-af2c-657108f45506",
      "assigned_to_id": "45bf9aff-b660-4124-b490-028f12dc5888",
      "created_at": "2026-06-12T07:22:11.609211Z",
      "updated_at": "2026-06-12T07:22:11.630781Z"
    },
    {
      "id": 10,
      "title": "Recover GoldenEye control keys",
      "description": "Obtain keys from Severnaya base.",
      "status": "todo",
      "priority": "high",
      "created_by_id": "8193154c-76a1-4c86-af2c-657108f45506",
      "assigned_to_id": "45bf9aff-b660-4124-b490-028f12dc5888",
      "created_at": "2026-06-12T07:22:11.598063Z",
      "updated_at": "2026-06-12T07:22:11.630781Z"
    }
  ],
  "cache_hit": true,
  "user": {
    "email": "james.bond@example.com",
    "role": "user"
  }
}
```
