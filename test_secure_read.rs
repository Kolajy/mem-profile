use std::io::Read;

fn read_securely(path: &str) -> String {
    let mut file = std::fs::File::open(path).unwrap_or_else(|e| panic!("Failed to open {}: {}", path, e));
    let meta = file.metadata().unwrap_or_else(|e| panic!("Failed to read metadata for {}: {}", path, e));
    if !meta.is_file() {
        panic!("{} is not a regular file", path);
    }
    let max_size = 256 * 1024 * 1024; // 256 MB
    if meta.len() > max_size {
        panic!("{} exceeds maximum allowed size (256 MB)", path);
    }
    let mut content = String::new();
    file.take(max_size).read_to_string(&mut content).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e));
    content
}

fn main() {
    let content = read_securely("test_secure_read.rs");
    println!("Read {} bytes", content.len());
}
