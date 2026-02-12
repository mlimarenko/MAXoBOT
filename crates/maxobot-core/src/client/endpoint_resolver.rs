use thiserror::Error;
use url::Url;

/// Canonical default API host used by MAX public documentation.
pub const DEFAULT_API_HOST: &str = "platform-api.max.ru";

/// Canonical default API base URL for MAX API calls.
pub const DEFAULT_API_BASE_URL: &str = "https://platform-api.max.ru";

/// Errors produced during endpoint/base URL validation and resolution.
#[derive(Debug, Error)]
pub enum EndpointResolverError {
    /// Base URL string could not be parsed.
    #[error("API base URL '{input}' is invalid")]
    InvalidBaseUrl {
        /// The raw base URL input that failed to parse.
        input: String,
        /// Parse error returned by `url`.
        #[source]
        source: url::ParseError,
    },
    /// Endpoint path could not be resolved against base URL.
    #[error("endpoint path '{path}' cannot be resolved")]
    InvalidEndpointPath {
        /// The endpoint path argument.
        path: String,
        /// Parse error returned by `url`.
        #[source]
        source: url::ParseError,
    },
    /// Base URL was parsed but does not have a host segment.
    #[error("API base URL must include a host")]
    MissingHost,
    /// Base URL is not HTTPS.
    #[error("API base URL must use https, got '{scheme}'")]
    NonHttpsScheme {
        /// Found URL scheme.
        scheme: String,
    },
}

/// Resolves API endpoint URLs from a validated API base URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointResolver {
    base_url: Url,
}

impl EndpointResolver {
    /// Creates a resolver for the canonical MAX API base URL.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a resolver from an explicit override string.
    pub fn with_override(base_url: impl AsRef<str>) -> Result<Self, EndpointResolverError> {
        let parsed = parse_and_validate_api_base_url(base_url.as_ref())?;
        Ok(Self { base_url: parsed })
    }

    /// Creates a resolver from an optional override string.
    ///
    /// If `override_base_url` is `None`, the default MAX API base URL is used.
    pub fn from_optional_override(
        override_base_url: Option<&str>,
    ) -> Result<Self, EndpointResolverError> {
        match override_base_url {
            Some(url) => Self::with_override(url),
            None => Ok(Self::default()),
        }
    }

    /// Creates a resolver from a parsed URL value.
    pub fn from_url(base_url: Url) -> Result<Self, EndpointResolverError> {
        validate_api_base_url(&base_url)?;
        Ok(Self {
            base_url: normalize_base_url(base_url),
        })
    }

    /// Returns the validated base URL.
    #[must_use]
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Resolves an endpoint path into a full URL.
    ///
    /// Both `me` and `/me` forms are accepted.
    pub fn resolve(&self, path: &str) -> Result<Url, EndpointResolverError> {
        let normalized_path = path.trim_start_matches('/');
        self.base_url.join(normalized_path).map_err(|source| {
            EndpointResolverError::InvalidEndpointPath {
                path: path.to_owned(),
                source,
            }
        })
    }
}

impl Default for EndpointResolver {
    fn default() -> Self {
        Self::with_override(DEFAULT_API_BASE_URL)
            .expect("DEFAULT_API_BASE_URL must be a valid HTTPS URL")
    }
}

/// Parses and validates API base URL from string input.
pub fn parse_and_validate_api_base_url(input: &str) -> Result<Url, EndpointResolverError> {
    let parsed = Url::parse(input).map_err(|source| EndpointResolverError::InvalidBaseUrl {
        input: input.to_owned(),
        source,
    })?;
    validate_api_base_url(&parsed)?;
    Ok(normalize_base_url(parsed))
}

/// Validates HTTPS and host requirements for API base URL.
pub fn validate_api_base_url(base_url: &Url) -> Result<(), EndpointResolverError> {
    if base_url.host_str().is_none() {
        return Err(EndpointResolverError::MissingHost);
    }

    if base_url.scheme() != "https" {
        return Err(EndpointResolverError::NonHttpsScheme {
            scheme: base_url.scheme().to_owned(),
        });
    }

    Ok(())
}

fn normalize_base_url(mut base_url: Url) -> Url {
    if !base_url.path().ends_with('/') {
        let mut path = base_url.path().trim_end_matches('/').to_owned();
        path.push('/');
        base_url.set_path(&path);
    }
    base_url.set_query(None);
    base_url.set_fragment(None);
    base_url
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_API_BASE_URL, DEFAULT_API_HOST, EndpointResolver, EndpointResolverError,
        parse_and_validate_api_base_url,
    };

    #[test]
    fn default_resolver_uses_canonical_host() {
        let resolver = EndpointResolver::default();

        assert_eq!(resolver.base_url().as_str(), "https://platform-api.max.ru/");
        assert_eq!(resolver.base_url().host_str(), Some(DEFAULT_API_HOST));
        assert!(
            resolver
                .base_url()
                .as_str()
                .starts_with(DEFAULT_API_BASE_URL)
        );
    }

    #[test]
    fn override_accepts_https_url() {
        let resolver =
            EndpointResolver::with_override("https://sandbox.example.test/v1").expect("valid URL");

        assert_eq!(
            resolver.base_url().as_str(),
            "https://sandbox.example.test/v1/"
        );
    }

    #[test]
    fn override_rejects_non_https_url() {
        let result = EndpointResolver::with_override("http://platform-api.max.ru");

        assert!(matches!(
            result,
            Err(EndpointResolverError::NonHttpsScheme { .. })
        ));
    }

    #[test]
    fn override_rejects_missing_host() {
        let result = EndpointResolver::with_override("file:///tmp/maxobot");

        assert!(matches!(result, Err(EndpointResolverError::MissingHost)));
    }

    #[test]
    fn resolve_joins_path_for_both_path_forms() {
        let resolver =
            EndpointResolver::with_override("https://platform-api.max.ru/v1").expect("valid URL");

        let relative = resolver.resolve("me").expect("path should resolve");
        let absolute = resolver.resolve("/me").expect("path should resolve");

        assert_eq!(relative.as_str(), "https://platform-api.max.ru/v1/me");
        assert_eq!(absolute.as_str(), "https://platform-api.max.ru/v1/me");
    }

    #[test]
    fn parse_and_validate_rejects_invalid_url_input() {
        let result = parse_and_validate_api_base_url("not-a-url");

        assert!(matches!(
            result,
            Err(EndpointResolverError::InvalidBaseUrl { .. })
        ));
    }
}
