//! Blocking HTTP client with cancellation support.

use crate::http::models::*;
use anyhow::{bail, Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Execute a request. Poll `cancel` to abort early.
pub fn execute(spec: &RequestSpec, cancel: Arc<AtomicBool>) -> Result<ResponseData> {
    if cancel.load(Ordering::SeqCst) {
        return Ok(cancelled());
    }

    let url = spec.url.trim();
    if url.is_empty() {
        bail!("URL is empty");
    }
    let parsed = url::Url::parse(url).context("Invalid URL")?;
    let _ = parsed;

    let mut builder = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(spec.timeout_ms.max(1)))
        .danger_accept_invalid_certs(!spec.ssl_verify);

    if spec.follow_redirects {
        builder = builder.redirect(reqwest::redirect::Policy::limited(
            spec.max_redirects as usize,
        ));
    } else {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }

    let client = builder.build().context("Failed to build HTTP client")?;

    let method = reqwest::Method::from_bytes(spec.method.as_bytes())
        .unwrap_or(reqwest::Method::GET);

    let mut req = client.request(method, url);

    // Headers
    for h in &spec.headers {
        if h.enabled && !h.key.is_empty() {
            req = req.header(h.key.as_str(), h.value.as_str());
        }
    }

    // Auth
    match spec.auth.kind {
        AuthKind::Basic => {
            req = req.basic_auth(&spec.auth.basic_user, Some(&spec.auth.basic_pass));
        }
        AuthKind::Bearer => {
            if !spec.auth.bearer_token.is_empty() {
                req = req.header(
                    "Authorization",
                    format!("Bearer {}", spec.auth.bearer_token),
                );
            }
        }
        AuthKind::ApiKey => {
            if !spec.auth.api_key_name.is_empty() {
                match spec.auth.api_key_location {
                    ApiKeyLocation::Header => {
                        req = req.header(
                            spec.auth.api_key_name.as_str(),
                            spec.auth.api_key_value.as_str(),
                        );
                    }
                    ApiKeyLocation::Query => {
                        // Append as query param
                        let mut u = url::Url::parse(url)?;
                        u.query_pairs_mut()
                            .append_pair(&spec.auth.api_key_name, &spec.auth.api_key_value);
                        req = client.request(
                            reqwest::Method::from_bytes(spec.method.as_bytes())
                                .unwrap_or(reqwest::Method::GET),
                            u.as_str(),
                        );
                        for h in &spec.headers {
                            if h.enabled && !h.key.is_empty() {
                                req = req.header(h.key.as_str(), h.value.as_str());
                            }
                        }
                    }
                }
            }
        }
        AuthKind::None => {}
    }

    // Body
    req = apply_body(req, &spec.body)?;

    if cancel.load(Ordering::SeqCst) {
        return Ok(cancelled());
    }

    let start = Instant::now();
    let response = match req.send() {
        Ok(r) => r,
        Err(e) => {
            if cancel.load(Ordering::SeqCst) {
                return Ok(cancelled());
            }
            return Ok(ResponseData {
                status: 0,
                status_text: String::new(),
                duration_ms: start.elapsed().as_millis() as u64,
                size_bytes: 0,
                headers: Vec::new(),
                body_text: String::new(),
                is_json: false,
                cancelled: false,
                error: Some(format!("{}", e)),
            });
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("")
        .to_string();

    let mut headers = Vec::new();
    for (k, v) in response.headers().iter() {
        headers.push((
            k.as_str().to_string(),
            v.to_str().unwrap_or("").to_string(),
        ));
    }

    if cancel.load(Ordering::SeqCst) {
        return Ok(cancelled());
    }

    let bytes = response.bytes().context("Failed to read body")?;
    let size_bytes = bytes.len() as u64;
    let body_text = String::from_utf8_lossy(&bytes).to_string();
    let is_json = serde_json::from_str::<serde_json::Value>(&body_text).is_ok();

    Ok(ResponseData {
        status,
        status_text,
        duration_ms,
        size_bytes,
        headers,
        body_text,
        is_json,
        cancelled: false,
        error: None,
    })
}

fn apply_body(
    mut req: reqwest::blocking::RequestBuilder,
    body: &Body,
) -> Result<reqwest::blocking::RequestBuilder> {
    match body.kind {
        BodyKind::None => Ok(req),
        BodyKind::Raw => {
            if !body.raw_content_type.is_empty() {
                req = req.header("Content-Type", &body.raw_content_type);
            }
            Ok(req.body(body.raw_text.clone()))
        }
        BodyKind::FormUrlEncoded => {
            Ok(req.form(&body.form_fields))
        }
        BodyKind::Multipart => {
            let mut form = reqwest::blocking::multipart::Form::new();
            for part in &body.multipart {
                if part.is_file {
                    if part.file_path.is_empty() {
                        continue;
                    }
                    form = form
                        .file(part.name.clone(), &part.file_path)
                        .with_context(|| format!("Cannot read file {}", part.file_path))?;
                } else {
                    form = form.text(part.name.clone(), part.value.clone());
                }
            }
            Ok(req.multipart(form))
        }
        BodyKind::Binary => {
            if body.binary_path.is_empty() {
                return Ok(req);
            }
            let data = std::fs::read(&body.binary_path)
                .with_context(|| format!("Cannot read {}", body.binary_path))?;
            if !body.binary_content_type.is_empty() {
                req = req.header("Content-Type", &body.binary_content_type);
            }
            Ok(req.body(data))
        }
    }
}

fn cancelled() -> ResponseData {
    ResponseData {
        status: 0,
        status_text: String::new(),
        duration_ms: 0,
        size_bytes: 0,
        headers: Vec::new(),
        body_text: String::new(),
        is_json: false,
        cancelled: true,
        error: None,
    }
}

/// Build a curl command from a RequestSpec.
pub fn to_curl(spec: &RequestSpec, redact: bool) -> String {
    let mut parts = vec!["curl".to_string()];

    if spec.method.to_uppercase() != "GET" {
        parts.push("-X".into());
        parts.push(spec.method.to_uppercase());
    }

    let url = if !spec.url.is_empty() {
        format!("'{}'", spec.url.replace('\'', "'\\''"))
    } else {
        "''".into()
    };
    parts.push(url);

    for h in &spec.headers {
        if !h.enabled || h.key.is_empty() {
            continue;
        }
        let val = if redact && is_secret_header(&h.key) {
            "<redacted>".into()
        } else {
            h.value.replace('\'', "'\\''")
        };
        parts.push("-H".into());
        parts.push(format!("'{}: {}'", h.key, val));
    }

    match spec.auth.kind {
        AuthKind::Basic => {
            let pass = if redact {
                "<redacted>".into()
            } else {
                spec.auth.basic_pass.replace('\'', "'\\''")
            };
            parts.push("-u".into());
            parts.push(format!(
                "'{}:{}'",
                spec.auth.basic_user.replace('\'', "'\\''"),
                pass
            ));
        }
        AuthKind::Bearer => {
            let tok = if redact {
                "<redacted>".into()
            } else {
                spec.auth.bearer_token.replace('\'', "'\\''")
            };
            parts.push("-H".into());
            parts.push(format!("'Authorization: Bearer {}'", tok));
        }
        AuthKind::ApiKey => {
            let val = if redact {
                "<redacted>".into()
            } else {
                spec.auth.api_key_value.replace('\'', "'\\''")
            };
            if spec.auth.api_key_location == ApiKeyLocation::Header {
                parts.push("-H".into());
                parts.push(format!("'{}: {}'", spec.auth.api_key_name, val));
            }
            // Query API keys are already in the URL when sent; for copy we append note in header form
        }
        AuthKind::None => {}
    }

    if !spec.ssl_verify {
        parts.push("-k".into());
    }

    match spec.body.kind {
        BodyKind::Raw if !spec.body.raw_text.is_empty() => {
            let escaped = spec.body.raw_text.replace('\'', "'\\''");
            parts.push("-d".into());
            parts.push(format!("'{}'", escaped));
        }
        BodyKind::FormUrlEncoded => {
            for (k, v) in &spec.body.form_fields {
                parts.push("--data-urlencode".into());
                parts.push(format!("'{}={}'", k, v.replace('\'', "'\\''")));
            }
        }
        BodyKind::Multipart => {
            for p in &spec.body.multipart {
                if p.is_file {
                    // Do not embed file contents; note path only
                    parts.push("-F".into());
                    parts.push(format!(
                        "'{}=@{};type=application/octet-stream'",
                        p.name,
                        p.file_path.replace('\'', "'\\''")
                    ));
                } else {
                    parts.push("-F".into());
                    parts.push(format!(
                        "'{}={}'",
                        p.name,
                        p.value.replace('\'', "'\\''")
                    ));
                }
            }
        }
        _ => {}
    }

    parts.join(" ")
}

fn is_secret_header(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == "authorization" || n == "cookie" || n.contains("api-key") || n.contains("token")
}
