//! Video metadata operation.

use http::Method;

use crate::{
    api::client::BotApiClient,
    client::{api_paths, http_executor::HttpExecutor},
    errors::api_error::ApiError,
    models::video::VideoMetadata,
};

impl<E> BotApiClient<E>
where
    E: HttpExecutor,
{
    /// Gets video metadata via `GET /videos/{videoToken}`.
    pub async fn get_video(&self, video_token: &str) -> Result<VideoMetadata, ApiError> {
        let path = api_paths::video(video_token)
            .map_err(|error| ApiError::InvalidConfiguration(error.to_string()))?;
        let request = self.request_builder(Method::GET, path)?.build()?;
        self.execute_json(request).await
    }
}
