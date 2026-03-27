use serde_json::{json, Value};

use crate::error::AppError;

pub fn success(command: &str, data: Value) -> Value {
    json!({
        "ok": true,
        "command": command,
        "data": data
    })
}

pub fn error_json(command: &str, err: &AppError) -> Value {
    json!({
        "ok": false,
        "command": command,
        "error": {
            "code": err.error_code(),
            "message": err.to_string(),
            "suggestion": err.suggestion(),
        }
    })
}

pub fn print_json(value: &Value) {
    println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
}

pub fn print_result(command: &str, result: Result<Value, AppError>, human: bool) {
    match result {
        Ok(data) => {
            let output = success(command, data);
            if human {
                print_human(&output);
            } else {
                print_json(&output);
            }
        }
        Err(err) => {
            let exit_code = err.exit_code();
            let output = error_json(command, &err);
            print_json(&output);
            std::process::exit(exit_code);
        }
    }
}

fn print_human(value: &Value) {
    if let Some(data) = value.get("data") {
        if let Some(obj) = data.as_object() {
            for (key, val) in obj {
                eprintln!("{}: {}", key, format_value(val));
            }
        } else if let Some(arr) = data.as_array() {
            for item in arr {
                eprintln!("{}", serde_json::to_string_pretty(item).unwrap_or_default());
                eprintln!("---");
            }
        } else {
            eprintln!("{}", data);
        }
    }
}

fn format_value(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}
