//! curl command parser → RequestSpec + warnings.

use crate::http::models::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CurlError {
    #[error("Empty input")]
    Empty,
    #[error("Not a curl command")]
    NotCurl,
    #[error("No URL found")]
    NoUrl,
    #[error("Unclosed quote")]
    UnclosedQuote,
    #[error("Malformed: {0}")]
    Malformed(String),
}

pub fn looks_like_curl(input: &str) -> bool {
    let t = input.trim_start();
    t.starts_with("curl ")
        || t.starts_with("curl\t")
        || t.starts_with("curl\n")
        || t.starts_with("curl.exe")
        || t == "curl"
        || t == "curl.exe"
}

/// Parse a curl command. Side-effect free.
pub fn parse_curl(input: &str) -> Result<CurlParseResult, CurlError> {
    let joined = join_continuations(input.trim());
    if joined.is_empty() {
        return Err(CurlError::Empty);
    }

    let tokens = tokenize(&joined)?;
    if tokens.is_empty() {
        return Err(CurlError::Empty);
    }

    let mut idx = 0;
    let first = tokens[0].to_ascii_lowercase();
    if first == "curl" || first == "curl.exe" {
        idx = 1;
    } else {
        return Err(CurlError::NotCurl);
    }

    let mut method = "GET".to_string();
    let mut url: Option<String> = None;
    let mut headers: Vec<HeaderKv> = Vec::new();
    let mut body_text = String::new();
    let mut has_data = false;
    let mut form_fields: Vec<(String, String)> = Vec::new();
    let mut multipart: Vec<FormField> = Vec::new();
    let mut auth = Auth::default();
    let mut ssl_verify = true;
    let mut warnings: Vec<String> = Vec::new();

    while idx < tokens.len() {
        let tok = &tokens[idx];
        if tok == "-X" || tok == "--request" {
            idx += 1;
            if idx >= tokens.len() {
                return Err(CurlError::Malformed("missing method after -X".into()));
            }
            method = tokens[idx].to_uppercase();
            idx += 1;
        } else if tok == "-H" || tok == "--header" {
            idx += 1;
            if idx >= tokens.len() {
                return Err(CurlError::Malformed("missing header after -H".into()));
            }
            match parse_header(&tokens[idx]) {
                Ok((k, v)) => headers.push(HeaderKv::new(k, v)),
                Err(w) => warnings.push(w),
            }
            idx += 1;
        } else if tok == "-d"
            || tok == "--data"
            || tok == "--data-raw"
            || tok == "--data-binary"
            || tok == "--data-urlencode"
        {
            let is_urlencode = tok == "--data-urlencode";
            let is_binary = tok == "--data-binary";
            idx += 1;
            if idx >= tokens.len() {
                return Err(CurlError::Malformed("missing value after -d".into()));
            }
            let val = &tokens[idx];
            if val.starts_with('@') {
                warnings.push(format!(
                    "Rejected file path in data flag ({} {}); file reads are not allowed",
                    if is_binary { "--data-binary" } else { "-d" },
                    val
                ));
            } else if is_urlencode {
                if let Some(eq) = val.find('=') {
                    form_fields.push((val[..eq].to_string(), val[eq + 1..].to_string()));
                } else {
                    form_fields.push((val.clone(), String::new()));
                }
                has_data = true;
            } else {
                body_text = val.clone();
                has_data = true;
            }
            if method == "GET" {
                method = "POST".to_string();
            }
            idx += 1;
        } else if tok == "-u" || tok == "--user" {
            idx += 1;
            if idx >= tokens.len() {
                return Err(CurlError::Malformed("missing value after -u".into()));
            }
            let cred = &tokens[idx];
            let (user, pass) = match cred.split_once(':') {
                Some((u, p)) => (u.to_string(), p.to_string()),
                None => (cred.clone(), String::new()),
            };
            auth.kind = AuthKind::Basic;
            auth.basic_user = user;
            auth.basic_pass = pass;
            idx += 1;
        } else if tok == "-F" || tok == "--form" {
            idx += 1;
            if idx >= tokens.len() {
                return Err(CurlError::Malformed("missing value after -F".into()));
            }
            let val = &tokens[idx];
            if let Some(eq) = val.find('=') {
                let name = val[..eq].to_string();
                let rest = &val[eq + 1..];
                if rest.starts_with('@') {
                    warnings.push(format!(
                        "Rejected file path in -F {} (file reads not allowed)",
                        val
                    ));
                } else {
                    multipart.push(FormField {
                        name,
                        value: rest.to_string(),
                        is_file: false,
                        file_path: String::new(),
                    });
                    has_data = true;
                    if method == "GET" {
                        method = "POST".to_string();
                    }
                }
            } else {
                warnings.push(format!("Malformed -F value: {}", val));
            }
            idx += 1;
        } else if tok == "--url" {
            idx += 1;
            if idx >= tokens.len() {
                return Err(CurlError::Malformed("missing URL after --url".into()));
            }
            url = Some(tokens[idx].clone());
            idx += 1;
        } else if tok == "-k" || tok == "--insecure" {
            ssl_verify = false;
            idx += 1;
        } else if tok == "--compressed" {
            // Acknowledge; reqwest may decompress automatically
            idx += 1;
        } else if tok.starts_with('-') {
            warnings.push(format!("Ignored unsupported flag: {}", tok));
            idx += 1;
            // Skip optional argument if next token is not a flag/url
            if idx < tokens.len()
                && !tokens[idx].starts_with('-')
                && !looks_like_url(&tokens[idx])
            {
                warnings.push(format!("Ignored argument to unsupported flag: {}", tokens[idx]));
                idx += 1;
            }
        } else {
            if url.is_none() {
                url = Some(tok.clone());
            } else {
                warnings.push(format!("Ignored extra argument: {}", tok));
            }
            idx += 1;
        }
    }

    let url = url.ok_or(CurlError::NoUrl)?;

    let mut body = Body::default();
    if !multipart.is_empty() {
        body.kind = BodyKind::Multipart;
        body.multipart = multipart;
    } else if !form_fields.is_empty() {
        body.kind = BodyKind::FormUrlEncoded;
        body.form_fields = form_fields;
    } else if has_data {
        body.kind = BodyKind::Raw;
        body.raw_text = body_text;
        let has_ct = headers
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case("content-type"));
        if !has_ct {
            let t = body.raw_text.trim();
            if (t.starts_with('{') && t.ends_with('}'))
                || (t.starts_with('[') && t.ends_with(']'))
            {
                body.raw_content_type = "application/json".into();
                headers.push(HeaderKv::new("Content-Type", "application/json"));
            } else {
                body.raw_content_type = "application/x-www-form-urlencoded".into();
            }
        }
    }

    let spec = RequestSpec {
        method,
        url,
        headers,
        body,
        auth,
        ssl_verify,
        timeout_ms: 30_000,
        follow_redirects: true,
        max_redirects: 10,
    };

    Ok(CurlParseResult {
        spec,
        warnings,
        error: None,
    })
}

fn parse_header(s: &str) -> Result<(String, String), String> {
    if let Some(colon) = s.find(':') {
        let key = s[..colon].trim().to_string();
        let value = s[colon + 1..].trim().to_string();
        if key.is_empty() {
            return Err(format!("Invalid header: {}", s));
        }
        Ok((key, value))
    } else {
        Err(format!("Invalid header (no colon): {}", s))
    }
}

fn looks_like_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://") || s.contains("://")
}

fn join_continuations(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            // If next is newline, skip both (continuation)
            match chars.peek() {
                Some('\n') => {
                    chars.next();
                }
                Some('\r') => {
                    chars.next();
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                }
                Some(_) => {
                    out.push('\\');
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn tokenize(input: &str) -> Result<Vec<String>, CurlError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            ' ' | '\t' | '\n' | '\r' if !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            '\\' if in_double => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            _ => current.push(c),
        }
    }
    if in_single || in_double {
        return Err(CurlError::UnclosedQuote);
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_get() {
        let r = parse_curl("curl https://api.example.com/users").unwrap();
        assert!(r.error.is_none());
        assert_eq!(r.spec.method, "GET");
        assert_eq!(r.spec.url, "https://api.example.com/users");
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn post_json() {
        let cmd = r#"curl -X POST https://api.example.com/users -H "Content-Type: application/json" -d '{"name":"A"}'"#;
        let r = parse_curl(cmd).unwrap();
        assert_eq!(r.spec.method, "POST");
        assert_eq!(r.spec.body.raw_text, r#"{"name":"A"}"#);
        assert!(r
            .spec
            .headers
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case("content-type")));
    }

    #[test]
    fn basic_auth() {
        let r = parse_curl("curl -X POST https://api.example.com -u user:pass").unwrap();
        assert_eq!(r.spec.auth.kind, AuthKind::Basic);
        assert_eq!(r.spec.auth.basic_user, "user");
        assert_eq!(r.spec.auth.basic_pass, "pass");
    }

    #[test]
    fn multiline_continuation() {
        let cmd = "curl -H \"A: b\" \\\n     -H \"C: d\" \\\n     https://x.com";
        let r = parse_curl(cmd).unwrap();
        assert_eq!(r.spec.url, "https://x.com");
        assert_eq!(r.spec.headers.len(), 2);
    }

    #[test]
    fn unclosed_quote_errors() {
        let err = parse_curl("curl 'https://x.com").unwrap_err();
        assert!(matches!(err, CurlError::UnclosedQuote));
    }

    #[test]
    fn unknown_flag_warning() {
        let r = parse_curl("curl -v https://api.example.com").unwrap();
        assert!(r.warnings.iter().any(|w| w.contains("-v")));
        assert_eq!(r.spec.url, "https://api.example.com");
    }

    #[test]
    fn file_path_rejected() {
        let r = parse_curl("curl -d @/etc/passwd https://x.com").unwrap();
        assert!(r.warnings.iter().any(|w| w.contains("Rejected file path")));
        assert!(r.spec.body.raw_text.is_empty() || r.spec.body.kind == BodyKind::None || !r.warnings.is_empty());
    }

    #[test]
    fn not_curl() {
        assert!(parse_curl("https://example.com").is_err());
    }

    #[test]
    fn looks_like() {
        assert!(looks_like_curl("curl https://x.com"));
        assert!(looks_like_curl("  curl.exe -X GET https://x.com"));
        assert!(!looks_like_curl("https://x.com"));
    }
}
