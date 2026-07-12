use inferno::flamegraph::{from_reader, Options};
use std::io::Cursor;

fn main() {
    let mut opts = Options::default();
    let data = b"main;foo 10\nmain;bar 20\n";
    let mut out = Vec::new();
    from_reader(&mut opts, &mut Cursor::new(data), &mut out).unwrap();
    println!("{}", String::from_utf8_lossy(&out).chars().take(200).collect::<String>());
}
