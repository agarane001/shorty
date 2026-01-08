use crate::{errors::AuthError, store::user::UserRepository};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use tracing::instrument;

#[derive(Clone, Debug)]
pub struct AuthService {
    repo: UserRepository,
}

impl AuthService {
    pub fn new(repo: UserRepository) -> Self {
        Self { repo }
    }

    pub async fn register(&self, email: &str, password: &str) -> anyhow::Result<uuid::Uuid> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| anyhow::anyhow!("failed to hash password"))?
            .to_string();

        self.repo.create_user(email, &hash).await
    }

    #[instrument(
        name = "AuthService: Login attempt", 
        skip(self, password), 
        fields(user_email = %email)
    )]
    pub async fn login(&self, email: &str, password: &str) -> Result<uuid::Uuid, AuthError> {
        // 1. Fetch User
        let user = self.repo.find_by_email(email).await.map_err(|e| {
            tracing::error!("Database error during login: {:?}", e);
            AuthError::Internal
        })?;

        let user = match user {
            Some(u) => u,
            None => {
                tracing::warn!("Login failed: User not found");
                return Err(AuthError::WrongCredentials);
            }
        };

        // 2. Parse Hash
        let parsed_hash = PasswordHash::new(&user.password_hash).map_err(|e| {
            tracing::error!("Critial: Failed to parse password hash from DB: {:?}", e);
            AuthError::Internal
        })?;

        // 3. Verify Password
        if Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_err() 
        {
            tracing::warn!("Login failed: Invalid password provided");
            return Err(AuthError::WrongCredentials);
        }

        tracing::info!("User authenticated successfully");
        Ok(user.id)
    }

}
