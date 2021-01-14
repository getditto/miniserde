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
                for child in vec {
                    stack.push(child);
                }
            }
            Value::Map(map) => {
                for (_, child) in map {
                    stack.push(child);
                }
            }
            _ => {}
        }
    }
}
