fn main() {
    let json = "\"stack\":";
    let parts: Vec<&str> = json.split("\"stack\":").collect();
    let current = parts[1];

    let stack_start = current.find('"').unwrap_or(0) + 1;
    let stack = if let Some(sub) = current.get(stack_start..) {
        let end_offset = sub.find('"').unwrap_or(0);
        sub.get(..end_offset).unwrap_or("").to_string()
    } else {
        "".to_string()
    };
    println!("stack: '{}'", stack);
}
