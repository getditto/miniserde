use crate::json::Value;

pub fn safely(value: Value) {
    match value {
        Value::Array(_) | Value::Object(_) | Value::Tag(..) => {}
        _ => return,
    }

    let mut stack = Vec::new();
    stack.push(value);
    while let Some(value) = stack.pop() {
        match value {
            Value::Array(vec) => {
                stack.extend(vec);
            }
            Value::Object(map) => {
                stack.extend(map.values());
            }
            Value::Tag(_, boxed) => {
                stack.push(*boxed);
            }
            _ => {}
        }
    }
}
