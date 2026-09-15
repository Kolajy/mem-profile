use num_format::{Locale, Buffer};

fn main() {
    let mut buf = Buffer::default();
    buf.write_formatted(&1234567890, &Locale::en);
    println!("Formatted: {}", buf.as_str());
}
