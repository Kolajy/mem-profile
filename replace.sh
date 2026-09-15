#!/bin/bash
sed -i 's/\([a-zA-Z_][a-zA-Z0-9_]*\)\.to_formatted_string(&Locale::en)/{ let mut buf = num_format::Buffer::default(); buf.write_formatted(\&\1, \&Locale::en); buf.as_str().to_string() }/g' src/main.rs
