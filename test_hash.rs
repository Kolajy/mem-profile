use std::collections::HashMap;

fn main() {
    let mut hm = HashMap::new();
    let mut buffer = String::with_capacity(64);

    // Simulate hot loop
    for i in 0..10 {
        buffer.clear();
        use std::fmt::Write;
        let _ = write!(&mut buffer, "key_{}", i);

        if let Some(v) = hm.get_mut(buffer.as_str()) {
            *v += 1;
        } else {
            hm.insert(buffer.clone(), 1);
        }
    }
    println!("{:?}", hm);
}
