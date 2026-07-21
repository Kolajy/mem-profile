fn main() {
    let json = "{\"stack_overflow\":{\"size\":100,\"count\":1}}";
    let parts: Vec<&str> = json.split("\":{\"size\":").collect();
    println!("{:?}", parts);
    for i in 1..parts.len() {
        let prev = parts[i - 1];
        let key_start = prev.rfind('"').unwrap_or(0);
        let key = prev[key_start + 1..].to_string();
        println!("key: {}", key);
    }
}
