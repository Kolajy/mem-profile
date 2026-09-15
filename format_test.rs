use num_format::{Locale, Buffer, ToFormattedString};
use std::fmt::Write;

fn main() {
    let num: u64 = 1234567;
    // Approach 1 (heap alloc)
    let s = num.to_formatted_string(&Locale::en);
    println!("{}", s);

    // Approach 2 (stack alloc)
    let mut buf = Buffer::default();
    buf.write_formatted(&num, &Locale::en);
    println!("{}", buf.as_str());
}
