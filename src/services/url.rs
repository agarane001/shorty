use crate::{
    models::url::UrlModel,
    store::{CacheRepository, UrlRepository},
};
use nanoid::nanoid;
use tracing::instrument;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct UrlService {
    repo: UrlRepository,
    cache: CacheRepository,
}

impl UrlService {
    pub fn new(repo: UrlRepository, cache: CacheRepository) -> Self {
        Self { repo, cache }
    }

    pub async fn shorten(&self, long_url: &str, user_id: uuid::Uuid) -> anyhow::Result<String> {
        let short_code = nanoid!(8);

        // Save to DB first
        self.repo.store(&short_code, long_url, user_id).await?;
        // Optimistically cache it
        let _ = self.cache.set(&short_code, long_url).await;

        Ok(short_code)
    }
    #[instrument(name = "Service: Resolve url", skip(self))]
    pub async fn resolve(&self, short_code: &str) -> Option<String> {
        // 1. Try Cache
        if let Some(url) = self.cache.get(short_code).await {
            let s_code = short_code.to_string();
            let repo = self.repo.clone();
            let cache = self.cache.clone();

            tokio::spawn(async move {
                // We must fetch the owner to know which cache to delete
                if let Ok(Some((_, Some(uid)))) = repo.fetch_with_owner(&s_code).await {
                    let _ = cache.delete_user_urls(uid).await;
                }
            });
            return Some(url);
        } else {
            tracing::warn!("Url not in cache!");
        }

        // 2. Try DB (fetch_with_owner ALREADY increments clicks)
        if let Ok(Some((url, user_id))) = self.repo.fetch_with_owner(short_code).await {
            let cache = self.cache.clone();

            // No need to call increment_clicks here! fetch_with_owner did it.
            tokio::spawn(async move {
                if let Some(uid) = user_id {
                    let _ = cache.delete_user_urls(uid).await;
                }
            });

            // Backfill individual link cache
            let _ = self.cache.set(short_code, &url).await;
            return Some(url);
        }
        tracing::warn!("Url was not found");

        None
    }

    pub async fn get_user_urls(&self, user_id: Uuid) -> anyhow::Result<Vec<UrlModel>> {
        self.repo.list_by_user(user_id).await
    }
}
