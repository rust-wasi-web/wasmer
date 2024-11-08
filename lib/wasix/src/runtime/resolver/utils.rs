use std::path::Path;

use anyhow::Error;
use http::{HeaderMap, StatusCode};
use url::Url;

use crate::http::{HttpResponse, USER_AGENT};

/// Polyfill for [`Url::from_file_path()`] that works on `wasm32-unknown-unknown`.
pub(crate) fn url_from_file_path(path: impl AsRef<Path>) -> Option<Url> {
    let path = path.as_ref();

    if !path.is_absolute() {
        return None;
    }

    let mut buffer = String::new();

    for component in path {
        if !buffer.ends_with('/') {
            buffer.push('/');
        }

        buffer.push_str(component.to_str()?);
    }

    buffer.insert_str(0, "file://");

    buffer.parse().ok()
}

pub(crate) fn webc_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("Accept", "application/webc".parse().unwrap());
    headers.insert("User-Agent", USER_AGENT.parse().unwrap());
    headers
}

pub(crate) fn http_error(response: &HttpResponse) -> Error {
    let status = response.status;

    if status == StatusCode::SERVICE_UNAVAILABLE {
        if let Some(retry_after) = response
            .headers
            .get("Retry-After")
            .and_then(|retry_after| retry_after.to_str().ok())
        {
            tracing::debug!(
                %retry_after,
                "Received 503 Service Unavailable while looking up a package. The backend may still be generating the *.webc file.",
            );
            return anyhow::anyhow!("{status} (Retry After: {retry_after})");
        }
    }

    Error::msg(status)
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    #[cfg(unix)]
    fn from_file_path_behaviour_is_identical() {
        let inputs = [
            "/",
            "/path",
            "/path/to/file.txt",
            "./path/to/file.txt",
            ".",
            "",
        ];

        for path in inputs {
            let got = url_from_file_path(path);
            let expected = Url::from_file_path(path).ok();
            assert_eq!(got, expected, "Mismatch for \"{path}\"");
        }
    }

    #[test]
    #[cfg(windows)]
    fn to_file_path_can_handle_unc_paths() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .unwrap();
        let url = Url::from_file_path(&path).unwrap();

        let got = file_path_from_url(&url).unwrap();

        assert_eq!(got.canonicalize().unwrap(), path);
    }
}
