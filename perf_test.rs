use std::time::Instant;

fn main() {
    let mut hm = std::collections::HashMap::new();
    let mut buffer = String::with_capacity(64);

    let start = Instant::now();
    for i in 0..1000000 {
        buffer.clear();
        use std::fmt::Write;
        let _ = write!(&mut buffer, "key_{}", i % 100);

        if let Some(v) = hm.get_mut(buffer.as_str()) {
            *v += 1;
        } else {
            hm.insert(buffer.clone(), 1);
        }
    }
    println!("get_mut/insert: {:?}", start.elapsed());

    let mut hm = std::collections::HashMap::new();
    let start = Instant::now();
    for i in 0..1000000 {
        buffer.clear();
        use std::fmt::Write;
        let _ = write!(&mut buffer, "key_{}", i % 100);

        *hm.entry(buffer.clone()).or_insert(0) += 1;
    }
    println!("entry: {:?}", start.elapsed());
}
