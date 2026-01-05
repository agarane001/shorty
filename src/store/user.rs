use crate::models::user::UserModel;
use sqlx::{Pool, Postgres};
use tracing::instrument;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct UserRepository {
    pool: Pool<Postgres>,
}

impl UserRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
    #[instrument(name = "Saving new user to database", skip(self, password_hash))]
    pub async fn create_user(&self, email: &str, password_hash: &str) -> anyhow::Result<Uuid> {
        let rec = sqlx::query!(
            "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
            email,
            password_hash
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
        Ok(rec.id)
    }

    #[instrument(name = "Fetching user by email from database", skip(self))]
    pub async fn find_by_email(&self, email: &str) -> anyhow::Result<Option<UserModel>> {
        let user = sqlx::query_as::<_, UserModel>(
            r#"SELECT id, email, password_hash, created_at FROM users WHERE email = $1"#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch user: {:?}", e);
            e
        })?;
        Ok(user)
    }

    pub async fn create_user_old(&self, email: &str, password_hash: &str) -> anyhow::Result<Uuid> {
        let rec = sqlx::query!(
            "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
            email,
            password_hash
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(rec.id)
    }

    pub async fn find_by_email_old(&self, email: &str) -> anyhow::Result<Option<UserModel>> {
        let user = sqlx::query_as::<_,UserModel>(
            r#"SELECT id, email, password_hash, created_at AS "created_at!: DateTime<Utc>" FROM users WHERE email = $1"#,
                    )
                        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }
}
