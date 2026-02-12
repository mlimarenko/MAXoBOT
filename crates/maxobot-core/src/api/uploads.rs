//! Upload ticket operations.

use http::Method;
use serde::{Deserialize, Serialize};

use crate::{
    api::client::BotApiClient,
    client::{api_paths, http_executor::HttpExecutor},
    errors::api_error::ApiError,
    models::upload_ticket::UploadTicket,
};

/// Supported upload ticket types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadType {
    /// Image upload ticket.
    Image,
    /// Video upload ticket.
    Video,
    /// Audio upload ticket.
    Audio,
    /// Generic file upload ticket.
    File,
}

impl UploadType {
    /// Returns upload type string used in query parameter.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::File => "file",
        }
    }
}

impl<E> BotApiClient<E>
where
    E: HttpExecutor,
{
    /// Creates an upload ticket via `POST /uploads?type=...`.
    pub async fn create_upload_ticket(
        &self,
        upload_type: UploadType,
    ) -> Result<UploadTicket, ApiError> {
        let request = self
            .request_builder(Method::POST, api_paths::UPLOADS)?
            .with_query_param("type", upload_type.as_str())
            .build()?;

        self.execute_json(request).await
    }
}
