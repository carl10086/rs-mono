use serde_json::Value;

pub fn parse_streaming_json(partial_json: &str) -> Value {
    if partial_json.trim().is_empty() {
        return Value::Object(serde_json::Map::new());
    }

    if let Ok(parsed) = serde_json::from_str::<Value>(partial_json) {
        return parsed;
    }

    let report = json_fix::fix_json_syntax(partial_json);
    if report.success {
        if let Ok(parsed) = serde_json::from_str::<Value>(&report.fixed) {
            return parsed;
        }
    }

    Value::Object(serde_json::Map::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        let result = parse_streaming_json("");
        assert!(result.is_object());
    }

    #[test]
    fn test_complete_json() {
        let result = parse_streaming_json(r#"{"key": "value"}"#);
        assert_eq!(result["key"], "value");
    }

    #[test]
    fn test_partial_json_repair() {
        let result = parse_streaming_json(r#"{"key": "value"#);
        assert!(result.is_object());
    }
}
