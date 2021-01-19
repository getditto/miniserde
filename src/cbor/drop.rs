use super::Value;

pub fn safely(value: Value) {
    match value {
        Value::Array(_) | Value::Map(_) => {}
        _ => return,
    }

    let mut stack = Vec::new();
    stack.push(value);
    while let Some(value) = stack.pop() {
        match value {
            Value::Array(vec) => {
                stack.extend(vec);
            }
            Value::Map(map) => {
                for (child_key, child_value) in map {
                    stack.push(child_key);
                    stack.push(child_value);
                }
            }
            _ => {}
        }
    }
}
