//! Request and response data models.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderKv {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

impl HeaderKv {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BodyKind {
    None,
    Raw,
    FormUrlEncoded,
    Multipart,
    Binary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub value: String,
    pub is_file: bool,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body {
    pub kind: BodyKind,
    pub raw_text: String,
    pub raw_content_type: String, // application/json, text/xml, text/html, text/plain
    pub form_fields: Vec<(String, String)>,
    pub multipart: Vec<FormField>,
    pub binary_path: String,
    pub binary_content_type: String,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            kind: BodyKind::None,
            raw_text: String::new(),
            raw_content_type: "application/json".into(),
            form_fields: Vec::new(),
            multipart: Vec::new(),
            binary_path: String::new(),
            binary_content_type: "application/octet-stream".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthKind {
    None,
    Basic,
    Bearer,
    ApiKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApiKeyLocation {
    Header,
    Query,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    pub kind: AuthKind,
    pub basic_user: String,
    pub basic_pass: String,
    pub bearer_token: String,
    pub api_key_name: String,
    pub api_key_value: String,
    pub api_key_location: ApiKeyLocation,
}

impl Default for Auth {
    fn default() -> Self {
        Self {
            kind: AuthKind::None,
            basic_user: String::new(),
            basic_pass: String::new(),
            bearer_token: String::new(),
            api_key_name: "X-API-Key".into(),
            api_key_value: String::new(),
            api_key_location: ApiKeyLocation::Header,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSpec {
    pub method: String,
    pub url: String,
    pub headers: Vec<HeaderKv>,
    pub body: Body,
    pub auth: Auth,
    pub ssl_verify: bool,
    pub timeout_ms: u64,
    pub follow_redirects: bool,
    pub max_redirects: u8,
}

impl Default for RequestSpec {
    fn default() -> Self {
        Self {
            method: "GET".into(),
            url: String::new(),
            headers: Vec::new(),
            body: Body::default(),
            auth: Auth::default(),
            ssl_verify: true,
            timeout_ms: 30_000,
            follow_redirects: true,
            max_redirects: 10,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResponseData {
    pub status: u16,
    pub status_text: String,
    pub duration_ms: u64,
    pub size_bytes: u64,
    pub headers: Vec<(String, String)>,
    pub body_text: String,
    pub is_json: bool,
    pub cancelled: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CurlParseResult {
    pub spec: RequestSpec,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: String,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedRequest {
    pub id: String,
    pub name: String,
    pub spec_json: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub redact_on_copy: bool,
    pub copy_mode: String, // js | jsonpath | variable
    pub timeout_ms: u64,
    pub follow_redirects: bool,
    pub max_redirects: u8,
    pub history_retention_days: u32,
    pub warned_clipboard_secrets: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            redact_on_copy: false,
            copy_mode: "js".into(),
            timeout_ms: 30_000,
            follow_redirects: true,
            max_redirects: 10,
            history_retention_days: 30,
            warned_clipboard_secrets: false,
        }
    }
}

/// Common header names for autocomplete.
pub const COMMON_HEADERS: &[&str] = &[
    "Accept",
    "Accept-Encoding",
    "Authorization",
    "Cache-Control",
    "Content-Type",
    "Cookie",
    "User-Agent",
    "X-Requested-With",
    "X-API-Key",
];
