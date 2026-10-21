//! Path generation for JSON click-to-copy (JS / JSONPath / variable modes).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyMode {
    Js,
    JsonPath,
    Variable,
}

impl CopyMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "jsonpath" => Self::JsonPath,
            "variable" => Self::Variable,
            _ => Self::Js,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Js => "js",
            Self::JsonPath => "jsonpath",
            Self::Variable => "variable",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Js => "JavaScript path",
            Self::JsonPath => "JSONPath",
            Self::Variable => "With variable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment {
    pub key: String,
    pub is_index: bool,
}

impl PathSegment {
    pub fn key(k: impl Into<String>) -> Self {
        Self {
            key: k.into(),
            is_index: false,
        }
    }
    pub fn index(i: usize) -> Self {
        Self {
            key: i.to_string(),
            is_index: true,
        }
    }
}

/// Valid JS identifier: /^[A-Za-z_$][A-Za-z0-9_$]*$/
pub fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

pub fn build_path(segments: &[PathSegment], mode: CopyMode, variable_name: &str) -> String {
    if segments.is_empty() {
        return match mode {
            CopyMode::Js => String::new(),
            CopyMode::JsonPath => "$".into(),
            CopyMode::Variable => variable_name.to_string(),
        };
    }

    let mut out = String::new();
    match mode {
        CopyMode::Js => {}
        CopyMode::JsonPath => out.push('$'),
        CopyMode::Variable => out.push_str(variable_name),
    }

    for (i, seg) in segments.iter().enumerate() {
        if seg.is_index {
            out.push('[');
            out.push_str(&seg.key);
            out.push(']');
        } else if is_valid_identifier(&seg.key) {
            // Leading segment in pure JS mode has no leading dot prefix object —
            // paths always start from root keys like `data...`
            if mode == CopyMode::Js && i == 0 {
                out.push_str(&seg.key);
            } else {
                out.push('.');
                out.push_str(&seg.key);
            }
        } else {
            // Bracket + double quotes
            if mode == CopyMode::Js && i == 0 {
                // rare root non-identifier
                out.push_str(&format!("[\"{}\"]", escape_js_string(&seg.key)));
            } else {
                out.push_str(&format!("[\"{}\"]", escape_js_string(&seg.key)));
            }
        }
    }
    out
}

fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segs(parts: &[(&str, bool)]) -> Vec<PathSegment> {
        parts
            .iter()
            .map(|(k, idx)| PathSegment {
                key: (*k).into(),
                is_index: *idx,
            })
            .collect()
    }

    #[test]
    fn basic_nesting() {
        let s = segs(&[("data", false), ("item", false), ("name", false)]);
        assert_eq!(build_path(&s, CopyMode::Js, "res"), "data.item.name");
    }

    #[test]
    fn array_index() {
        let s = segs(&[
            ("data", false),
            ("items", false),
            ("0", true),
            ("name", false),
        ]);
        assert_eq!(build_path(&s, CopyMode::Js, "res"), "data.items[0].name");
    }

    #[test]
    fn nested_arrays() {
        let s = segs(&[
            ("data", false),
            ("matrix", false),
            ("0", true),
            ("1", true),
        ]);
        assert_eq!(build_path(&s, CopyMode::Js, "res"), "data.matrix[0][1]");
    }

    #[test]
    fn key_with_space() {
        let s = segs(&[("data", false), ("user name", false)]);
        assert_eq!(
            build_path(&s, CopyMode::Js, "res"),
            "data[\"user name\"]"
        );
    }

    #[test]
    fn key_with_hyphen() {
        let s = segs(&[("data", false), ("2fa-enabled", false)]);
        assert_eq!(
            build_path(&s, CopyMode::Js, "res"),
            "data[\"2fa-enabled\"]"
        );
    }

    #[test]
    fn key_with_dot() {
        let s = segs(&[("data", false), ("a.b", false)]);
        assert_eq!(build_path(&s, CopyMode::Js, "res"), "data[\"a.b\"]");
    }

    #[test]
    fn numeric_string_key() {
        let s = segs(&[("data", false), ("0", false)]);
        assert_eq!(build_path(&s, CopyMode::Js, "res"), "data[\"0\"]");
    }

    #[test]
    fn jsonpath_mode() {
        let s = segs(&[
            ("data", false),
            ("items", false),
            ("0", true),
            ("name", false),
        ]);
        assert_eq!(
            build_path(&s, CopyMode::JsonPath, "res"),
            "$.data.items[0].name"
        );
    }

    #[test]
    fn variable_mode() {
        let s = segs(&[
            ("data", false),
            ("items", false),
            ("0", true),
            ("name", false),
        ]);
        assert_eq!(
            build_path(&s, CopyMode::Variable, "res"),
            "res.data.items[0].name"
        );
    }

    #[test]
    fn empty_object_path() {
        let s = segs(&[("data", false), ("empty", false)]);
        assert_eq!(build_path(&s, CopyMode::Js, "res"), "data.empty");
    }

    #[test]
    fn null_value_path() {
        let s = segs(&[("data", false), ("value", false)]);
        assert_eq!(build_path(&s, CopyMode::Js, "res"), "data.value");
    }

    /// Spec example 1: clicking "sdfsdf"
    #[test]
    fn spec_example_nested() {
        let s = segs(&[
            ("data", false),
            ("item", false),
            ("nestedItem", false),
            ("nestedNestedItem", false),
        ]);
        assert_eq!(
            build_path(&s, CopyMode::Js, "res"),
            "data.item.nestedItem.nestedNestedItem"
        );
    }

    /// Spec example 2: clicking "API tool"
    #[test]
    fn spec_example_api_tool() {
        let s = segs(&[
            ("data", false),
            ("items", false),
            ("0", true),
            ("details", false),
            ("category", false),
        ]);
        assert_eq!(
            build_path(&s, CopyMode::Js, "res"),
            "data.items[0].details.category"
        );
    }

    /// Spec example 2: clicking "Postman"
    #[test]
    fn spec_example_postman() {
        let s = segs(&[
            ("data", false),
            ("items", false),
            ("0", true),
            ("name", false),
        ]);
        assert_eq!(
            build_path(&s, CopyMode::Js, "res"),
            "data.items[0].name"
        );
    }
}
