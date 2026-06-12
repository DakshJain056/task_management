use redis::AsyncCommands;
use uuid::Uuid;

use crate::models::TaskViewResponse;

#[derive(Clone)]
pub struct TaskCache {
    connection_manager: redis::aio::ConnectionManager,
}

impl std::fmt::Debug for TaskCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskCache").finish()
    }
}

impl TaskCache {
    /// Initialize a new Redis-backed cache.
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        let connection_manager = redis::aio::ConnectionManager::new(client).await?;
        Ok(Self { connection_manager })
    }

    /// Retrieve the cached TaskViewResponse for a user.
    pub async fn get(&self, user_id: &Uuid) -> Option<TaskViewResponse> {
        let mut conn = self.connection_manager.clone();
        let key = format!("tasks:user:{}", user_id);
        
        let cached_str: Option<String> = conn.get(&key).await.ok().flatten();
        if let Some(json_str) = cached_str {
            serde_json::from_str(&json_str).ok()
        } else {
            None
        }
    }

    /// Cache the TaskViewResponse for a user.
    pub async fn set(&self, user_id: Uuid, mut response: TaskViewResponse) {
        // Ensure that cached responses are marked with cache_hit = true when read later
        response.cache_hit = true;
        let mut conn = self.connection_manager.clone();
        let key = format!("tasks:user:{}", user_id);
        
        if let Ok(json_str) = serde_json::to_string(&response) {
            // Cache expires after 10 minutes (600 seconds)
            let _: () = conn.set_ex(&key, json_str, 600).await.unwrap_or(());
        }
    }

    /// Remove a specific user's cached tasks.
    pub async fn invalidate(&self, user_id: &Uuid) {
        let mut conn = self.connection_manager.clone();
        let key = format!("tasks:user:{}", user_id);
        let _: () = conn.del(&key).await.unwrap_or(());
    }

    /// Clear all keys from the Redis cache.
    pub async fn clear(&self) {
        let mut conn = self.connection_manager.clone();
        let _: () = redis::cmd("FLUSHDB")
            .query_async(&mut conn)
            .await
            .unwrap_or(());
    }
}
