// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Embedded static assets for the single-binary web interface.
//!
//! Assets are compiled into the binary with `include_str!`. There are no
//! external requests — no CDN fonts, scripts, or telemetry — which keeps
//! the air-gap invariant intact.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use axum::extract::Path;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};

const INDEX: &str = include_str!("../../../site/app/index.html");
const CSS: &str = include_str!("../../../site/app/app.css");

/// Frontend ES modules, embedded and served only from `/assets/js/`.
pub const JS_MODULES: &[(&str, &str)] = &[
    ("main.js", include_str!("../../../site/app/js/main.js")),
    ("api.js", include_str!("../../../site/app/js/api.js")),
    ("argv.js", include_str!("../../../site/app/js/argv.js")),
    ("icons.js", include_str!("../../../site/app/js/icons.js")),
    ("format.js", include_str!("../../../site/app/js/format.js")),
    ("surface.js", include_str!("../../../site/app/js/surface.js")),
    ("terminal.js", include_str!("../../../site/app/js/terminal.js")),
    ("ledger.js", include_str!("../../../site/app/js/ledger.js")),
    ("palette.js", include_str!("../../../site/app/js/palette.js")),
];

/// Security headers applied to every response.
pub fn security_headers(headers: &mut HeaderMap) {
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self'; \
             img-src 'self' data:; connect-src 'self'; font-src 'self'; \
             object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
}

/// Serve the application shell.
pub async fn index() -> Response {
    let mut headers = HeaderMap::new();
    security_headers(&mut headers);
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    (headers, Html(INDEX)).into_response()
}

/// Serve the stylesheet.
pub async fn css() -> Response {
    let mut headers = HeaderMap::new();
    security_headers(&mut headers);
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/css; charset=utf-8"),
    );
    (headers, CSS).into_response()
}

/// Serve one embedded frontend module. Unknown names are rejected.
pub async fn javascript_module(Path(name): Path<String>) -> Response {
    let Some((_, source)) = JS_MODULES.iter().find(|(file, _)| *file == name) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    let mut headers = HeaderMap::new();
    security_headers(&mut headers);
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/javascript; charset=utf-8"),
    );
    (headers, *source).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_assets_are_present() {
        assert!(INDEX.contains("<!doctype html>"));
        assert!(CSS.contains("--bg"));
        assert!(JS_MODULES.len() >= 8);
        assert!(JS_MODULES.iter().all(|(_, source)| !source.is_empty()));
    }

    #[test]
    fn security_policy_blocks_external_loads() {
        let mut headers = HeaderMap::new();
        security_headers(&mut headers);
        let csp = headers
            .get(header::CONTENT_SECURITY_POLICY)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(csp.contains("default-src 'self'"));
        assert!(csp.contains("connect-src 'self'"));
        assert!(csp.contains("frame-ancestors 'none'"));
        assert!(!csp.contains("unsafe-inline"));
        assert!(!csp.contains("unsafe-eval"));
    }

    #[test]
    fn assets_reference_no_external_origins() {
        for (name, source) in std::iter::once(("index.html", INDEX))
            .chain(std::iter::once(("app.css", CSS)))
            .chain(JS_MODULES.iter().copied())
        {
            assert!(!source.contains("http://"), "{name} contains an external URL");
            assert!(!source.contains("https://"), "{name} contains an external URL");
        }
    }
}
