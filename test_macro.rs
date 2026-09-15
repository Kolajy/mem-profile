use num_format::{Locale, Buffer};

fn format_with_buffer(val: &impl num_format::ToFormattedStr) -> String {
    let mut buf = Buffer::default();
    buf.write_formatted(val, &Locale::en);
    buf.as_str().to_string()
}
