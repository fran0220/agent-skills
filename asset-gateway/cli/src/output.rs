use serde::Serialize;
use serde_json::{json, Value};

/// Build a success JSON envelope.
pub fn success(command: &str, data: impl Serialize) -> Value {
    json!({
        "ok": true,
        "command": command,
        "data": serde_json::to_value(data).unwrap_or(Value::Null),
    })
}

/// Build an error JSON envelope.
pub fn error(command: &str, code: &str, message: &str, suggestion: Option<&str>) -> Value {
    let mut err = json!({
        "code": code,
        "message": message,
    });
    if let Some(s) = suggestion {
        err["suggestion"] = Value::String(s.into());
    }
    json!({
        "ok": false,
        "command": command,
        "error": err,
    })
}

/// Print a JSON value to stdout.
pub fn print_json(val: &Value) {
    println!("{}", serde_json::to_string_pretty(val).unwrap_or_default());
}
