//! Bot method-group operations.

use http::Method;

use crate::{
    api::client::BotApiClient,
    client::{api_paths, http_executor::HttpExecutor},
    errors::api_error::ApiError,
    models::user::User,
};

impl<E> BotApiClient<E>
where
    E: HttpExecutor,
{
    /// Gets current bot profile via `GET /me`.
    pub async fn get_me(&self) -> Result<User, ApiError> {
        let request = self.request_builder(Method::GET, api_paths::ME)?.build()?;
        self.execute_json(request).await
    }
}
