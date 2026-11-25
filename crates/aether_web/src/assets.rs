// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Embedded static assets for the single-binary web interface.
//!
//! Assets are compiled into the binary with `include_str!`. There are no
//! external requests — no CDN fonts, scripts, or telemetry — which keeps
//! the air-gap invariant intact.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{Html, IntoResponse, Response};

const INDEX: &str = include_str!("../../../site/app/index.html");
const CSS: &str = include_str!("../../../site/app/app.css");
const JS: &str = include_str!("../../../site/app/app.js");

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

/// Serve the application module.
pub async fn javascript() -> Response {
    let mut headers = HeaderMap::new();
    security_headers(&mut headers);
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/javascript; charset=utf-8"),
    );
    (headers, JS).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_assets_are_present() {
        assert!(INDEX.contains("<!doctype html>"));
        assert!(CSS.contains("--bg"));
        assert!(!JS.is_empty());
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
    }
}