use redis::AsyncCommands;
use sqlx::{Pool, Postgres};
use tracing::instrument;
use uuid::Uuid;

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

    #[instrument(name = "Increment clicks")]
    pub async fn increment_clicks(&self, short_code: &str) -> anyhow::Result<()> {
        sqlx::query!(
            "UPDATE urls SET clicks = clicks + 1 WHERE short_code = $1",
            short_code
        )
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Fetch all URLs belonging to a specific user
    pub async fn list_by_user(&self, user_id: uuid::Uuid) -> anyhow::Result<Vec<UrlModel>> {
        let rows = sqlx::query_as::<_, UrlModel>(
            r#"SELECT short_code, long_url, user_id, clicks, created_at
            FROM urls
            WHERE user_id = $1 
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(rows)
    }
    pub async fn fetch_with_owner(
        &self,
        short_code: &str,
    ) -> anyhow::Result<Option<(String, Option<Uuid>)>> {
        let row = sqlx::query!(
            r#"UPDATE urls 
            SET clicks = clicks + 1
            WHERE short_code = $1
            RETURNING long_url, user_id;"#,
            short_code
        )
        .fetch_optional(&self.pg_pool)
        .await?;

        Ok(row.map(|r| (r.long_url, r.user_id)))
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

    pub async fn invalidate_stats(&self, user_id: Uuid) -> anyhow::Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let key = format!("user_urls:{}", user_id);
        let _: () = conn.del(key).await?;
        Ok(())
    }
    pub async fn delete_user_urls(&self, user_id: Uuid) -> anyhow::Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let key = format!("user_urls:{}", user_id);
        let _: () = conn.del(key).await?;
        Ok(())
    }

    pub async fn set_user_urls(&self, user_id: Uuid, urls: &[UrlModel]) -> anyhow::Result<()> {
        let mut conn = self.redis_pool.get().await?;
        let key = format!("user_urls:{}", user_id);
        let value = serde_json::to_string(urls)?;
        conn.set_ex::<String, String, u64>(key, value, 300).await?; // 5 min TTL
        Ok(())
    }
}
