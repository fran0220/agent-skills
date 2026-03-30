use serde_json::{json, Value};

pub fn success(command: &str, data: Value) -> Value {
    json!({"ok": true, "command": command, "data": data})
}

pub fn error(command: &str, code: &str, message: &str) -> Value {
    json!({"ok": false, "command": command, "error": {"code": code, "message": message}})
}

pub fn print_json(value: &Value) {
    println!("{}", serde_json::to_string(value).unwrap());
}

pub fn print_json_pretty(value: &Value) {
    println!("{}", serde_json::to_string_pretty(value).unwrap());
}
