//! Authorization helpers for MAX API requests.

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

use crate::errors::api_error::ApiError;

/// Query parameter name that is forbidden for auth in modern MAX API usage.
pub const ACCESS_TOKEN_QUERY_KEY: &str = "access_token";

/// Injects the MAX token into the `Authorization` header.
///
/// The MAX API expects the raw token value in this header.
pub fn inject_authorization_header(headers: &mut HeaderMap, token: &str) -> Result<(), ApiError> {
    if token.trim().is_empty() {
        return Err(ApiError::InvalidConfiguration(
            "token cannot be empty".to_owned(),
        ));
    }

    let value = HeaderValue::from_str(token).map_err(|error| ApiError::InvalidHeader {
        header: AUTHORIZATION.as_str().to_owned(),
        reason: error.to_string(),
    })?;

    headers.insert(AUTHORIZATION, value);
    Ok(())
}

/// Fails fast when legacy query-based auth is used.
pub fn ensure_no_query_auth(query: &[(String, String)]) -> Result<(), ApiError> {
    if query.iter().any(|(key, _)| key == ACCESS_TOKEN_QUERY_KEY) {
        return Err(ApiError::QueryAuthenticationForbidden);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use reqwest::header::HeaderMap;

    use super::{ensure_no_query_auth, inject_authorization_header};

    #[test]
    fn injects_authorization_header() {
        let mut headers = HeaderMap::new();
        inject_authorization_header(&mut headers, "token-value").expect("header must be injected");

        assert_eq!(headers["authorization"], "token-value");
    }

    #[test]
    fn rejects_query_auth() {
        let query = vec![
            ("chat_id".to_owned(), "1".to_owned()),
            ("access_token".to_owned(), "secret".to_owned()),
        ];

        let result = ensure_no_query_auth(&query);
        assert!(result.is_err());
    }
}
