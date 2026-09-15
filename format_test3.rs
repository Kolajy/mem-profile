use num_format::{Locale, Buffer};
use std::fmt::Write;

fn main() {
    let mut buf = Buffer::default();
    let num: usize = 12345;
    buf.write_formatted(&num, &Locale::en);
    println!("formatted: {}", buf.as_str());
}
