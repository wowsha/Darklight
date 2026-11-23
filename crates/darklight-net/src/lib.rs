//! Networking for Darklight.
//!
//! The URL Standard parser (`url`) and the HTTP/TLS stack (`ureq` + rustls) are
//! deliberate dependencies: networking and TLS are platform plumbing, not the
//! engine. Everything above this crate (parsing, style, layout, paint) is ours.

#![forbid(unsafe_code)]

use std::fmt;
use url::Url;

#[derive(Debug)]
pub enum FetchError {
    InvalidUrl(String),
    UnsupportedScheme(String),
    Transport(String),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::InvalidUrl(e) => write!(f, "invalid URL: {e}"),
            FetchError::UnsupportedScheme(s) => write!(f, "unsupported scheme: {s}"),
            FetchError::Transport(e) => write!(f, "network error: {e}"),
        }
    }
}

impl std::error::Error for FetchError {}

#[derive(Debug, Clone)]
pub struct Response {
    pub url: Url,
    pub status: u16,
    /// Header names are lower-cased by the HTTP stack.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Body decoded as UTF-8, replacing invalid sequences.
    /// Proper charset detection is planned for Phase 1.
    pub fn text_lossy(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Parses user input into a navigable URL. Input without a scheme is treated as
/// `https://`.
pub fn parse_input_url(input: &str) -> Result<Url, FetchError> {
    let input = input.trim();
    let candidate = if input.contains("://") {
        input.to_string()
    } else {
        format!("https://{input}")
    };
    let url = Url::parse(&candidate).map_err(|e| FetchError::InvalidUrl(e.to_string()))?;
    match url.scheme() {
        "http" | "https" => Ok(url),
        other => Err(FetchError::UnsupportedScheme(other.to_string())),
    }
}

/// Fetches `input` with a GET request (redirects followed, 4xx/5xx returned as
/// ordinary responses). Blocking; run it off the UI thread.
pub fn fetch(input: &str) -> Result<Response, FetchError> {
    let url = parse_input_url(input)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();
    let mut resp = agent
        .get(url.as_str())
        .call()
        .map_err(|e| FetchError::Transport(e.to_string()))?;
    let status = resp.status().as_u16();
    let headers = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let body = resp
        .body_mut()
        .read_to_vec()
        .map_err(|e| FetchError::Transport(e.to_string()))?;
    Ok(Response {
        url,
        status,
        headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_https_when_scheme_missing() {
        assert_eq!(
            parse_input_url("example.com/a").unwrap().as_str(),
            "https://example.com/a"
        );
    }

    #[test]
    fn keeps_explicit_http() {
        assert_eq!(
            parse_input_url(" http://example.com ").unwrap().scheme(),
            "http"
        );
    }

    #[test]
    fn rejects_other_schemes() {
        assert!(matches!(
            parse_input_url("file:///etc/passwd"),
            Err(FetchError::UnsupportedScheme(_))
        ));
        assert!(matches!(
            parse_input_url("ftp://example.com"),
            Err(FetchError::UnsupportedScheme(_))
        ));
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(
            parse_input_url("http://"),
            Err(FetchError::InvalidUrl(_))
        ));
    }
}
