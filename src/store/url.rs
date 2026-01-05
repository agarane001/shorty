use redis::AsyncCommands;
use sqlx::{Pool, Postgres};

use crate::models::url::UrlModel;

#[derive(Clone, Debug)]
pub struct UrlRepository {
    pg_pool: Pool<Postgres>,
}

impl UrlRepository {
    pub fn new(pg_pool: Pool<Postgres>) -> Self {
        Self { pg_pool }
    }

    pub async fn store(
        &self,
        short_code: &str,
        long_url: &str,
        user_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            "INSERT INTO urls (short_code, long_url, user_id) VALUES ($1, $2, $3)",
            short_code,
            long_url,
            user_id, /* Uuid */
        )
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn fetch(&self, short_code: &str) -> anyhow::Result<Option<String>> {
        let row = sqlx::query!(
            "SELECT long_url FROM urls WHERE short_code = $1",
            short_code
        )
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(row.map(|r| r.long_url))
    }

    /// Fetch all URLs belonging to a specific user
    pub async fn list_by_user(&self, user_id: uuid::Uuid) -> anyhow::Result<Vec<UrlModel>> {
        let rows = sqlx::query_as::<_, UrlModel>(
            r#"SELECT short_code, long_url, user_id, clicks, created_at AS "created_at!: DateTime<Utc>"
            FROM urls
            WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(rows)
    }
}

#[derive(Clone, Debug)]
pub struct CacheRepository {
    redis_pool: bb8::Pool<redis::Client>,
}

impl CacheRepository {
    pub fn new(redis_pool: bb8::Pool<redis::Client>) -> Self {
        Self { redis_pool }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let mut conn = self.redis_pool.get().await.ok()?;
        conn.get(key).await.ok()
    }

    pub async fn set(&self, key: &str, value: &str) -> anyhow::Result<()> {
        let mut conn = self.redis_pool.get().await?;
        conn.set_ex::<&str, &str, u64>(key, value, 3600).await?; // 1 hour TTL
        Ok(())
    }
}
