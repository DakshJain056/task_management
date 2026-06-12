# AI Usage Documentation

This document describes how the AI coding assistant was utilized during the development, testing, and operation of the Task Management Service.

---

## 🛠️ Code Generation & Architecture Design

The AI assistant helped architect and implement the entire backend layout:
1. **Framework Choices**: Set up the Axum async routing system, Tokio runtime, SQLx database operations, Argon2 password cryptography, and jsonwebtoken token-handling mechanisms.
2. **Schema Definition**: Wrote SQL migrations programmatically applying updates to Postgres (such as updating column names to `full_name`, `hashed_password`, `created_by_id`, `assigned_to_id`, etc.).
3. **Redis Caching integration**: Replaced the custom in-memory lock-protected cache layer with a Redis connection manager using JSON serialization (`serde_json`).

---

## 🧪 Automated Testing & Verification

The AI assistant automated the verification of all endpoints by creating and modifying scripts in the sandbox/scratch directory:
1. **Interactive test suites (`test_endpoints.py`)**: Scripted a full lifecycle of user seeding, multi-step 2FA login verification, challenge validity check, JWT extraction, and role authorization checks.
2. **Scenario walkthroughs (`scenario_test.py`)**: Automated the execution of the user-requested test scenario (admin creating 5 tasks, assigning 3 to James Bond, and James Bond checking tasks twice to test the cache `MISS` and `HIT` headers).

---

## 🐋 Environment & Process Management

1. **Dockerized Cache Setup**: Orchestrated the docker pulls and launch parameters to set up a clean Redis container instance on port `6379`.
2. **Dynamic Compilations**: Used cargo check and build loops to analyze syntax, implement type-correct async trait extractors (`FromRequestParts`), and resolve compiler trait constraints (e.g. implementing custom `std::fmt::Debug` for the cache struct due to non-debug redis connection managers).
3. **Graceful Terminations**: Managed process lifecycles by cleanly signaling background servers to shut down after completion of integration scripts to keep workspace resources free.
