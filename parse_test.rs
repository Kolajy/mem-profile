fn main() {
    let buf = b"2455 124 111 23 0 111 0";
    let n = buf.len();

    let mut state = 0;
    let mut pages: u64 = 0;
    let mut found = false;

    for &b in &buf[..n] {
        if b == b' ' {
            if state == 0 {
                state = 1;
            } else if state == 1 {
                break;
            }
        } else if state == 1 && b.is_ascii_digit() {
            pages = pages * 10 + (b - b'0') as u64;
            found = true;
        }
    }

    if found {
        println!("Pages: {}", pages);
    } else {
        println!("Not found");
    }
}
