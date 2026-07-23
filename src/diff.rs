use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Default)]
pub struct AllocationStats {
    pub size: usize,
    pub count: usize,
}

// Simple JSON parser for our specific format
// Format: [{"stack": "...", "size": 123, "count": 1}, ...]
// Or a dictionary format: {"main;foo": {"size": 100, "count": 1}}
// Let's assume the snapshot output is standard JSON map of stack to size and count
// We will write a tiny bespoke JSON parser to avoid adding serde dependencies

fn parse_json_map(json: &str) -> HashMap<String, AllocationStats> {
    let mut result = HashMap::new();

    // Simplistic parser: look for "stack", "size", "count" if array
    // Since we originally saved as HashMap<String, AllocationStats>, it'll be formatted like:
    // {"main;foo":{"size":100,"count":1},"main;bar":{"size":200,"count":2}}

    let mut in_string = false;
    let mut chars = json.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '"' {
            in_string = !in_string;
            if in_string {
                // Read string
                let mut s = String::new();
                while let Some(&c2) = chars.peek() {
                    if c2 == '"' {
                        chars.next();
                        break;
                    }
                    s.push(c2);
                    chars.next();
                }

                // If this is a key at the root level, we should expect a colon next
                // Let's do a simpler string matching instead of full character parsing
            }
        }
    }

    // An even simpler approach: regex or string find
    // Because we just need to parse {"key": {"size": X, "count": Y}}

    let parts: Vec<&str> = json.split("\":{\"size\":").collect();
    if parts.len() <= 1 {
        // Maybe it's formatted with spaces or different structure
        return parse_json_array(json);
    }

    for i in 1..parts.len() {
        // parts[i-1] ends with the key string
        let prev = parts[i - 1];
        let key_start = prev.rfind('"').unwrap_or(0);
        let key = prev.get(key_start + 1..).unwrap_or("").to_string();

        // parts[i] starts with the size, followed by ,"count":
        let current = parts[i];
        let size_end_idx = current.find(',').unwrap_or(current.len());
        let size_str = current.get(..size_end_idx).unwrap_or("").trim();
        let size = size_str.parse::<usize>().unwrap_or(0);

        let count_idx = current.find("\"count\":").unwrap_or(current.len());
        if count_idx < current.len() {
            if let Some(rem) = current.get(count_idx + 8..) {
                let count_end = rem.find('}').unwrap_or(rem.len());
                let count_str = rem.get(..count_end).unwrap_or("").trim();
                let count = count_str.parse::<usize>().unwrap_or(0);

                result.insert(key, AllocationStats { size, count });
            }
        }
    }

    result
}

// Format: [ {"stack": "...", "size": 123, "count": 1}, ... ]
fn parse_json_array(json: &str) -> HashMap<String, AllocationStats> {
    let mut result = HashMap::new();

    let parts: Vec<&str> = json.split("\"stack\":").collect();
    for i in 1..parts.len() {
        let current = parts[i];

        let stack_start = current.find('"').unwrap_or(0) + 1;
        let stack = if let Some(sub) = current.get(stack_start..) {
            let end_offset = sub.find('"').unwrap_or(0);
            sub.get(..end_offset).unwrap_or("").to_string()
        } else {
            "".to_string()
        };

        let size_idx = current.find("\"size\":").unwrap_or(current.len());
        let size = if size_idx < current.len() {
            if let Some(rem) = current.get(size_idx + 7..) {
                let end = rem.find(',').unwrap_or(rem.find('}').unwrap_or(rem.len()));
                rem.get(..end)
                    .unwrap_or("")
                    .trim()
                    .parse::<usize>()
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        let count_idx = current.find("\"count\":").unwrap_or(current.len());
        let count = if count_idx < current.len() {
            if let Some(rem) = current.get(count_idx + 8..) {
                let end = rem.find(',').unwrap_or(rem.find('}').unwrap_or(rem.len()));
                rem.get(..end)
                    .unwrap_or("")
                    .trim()
                    .parse::<usize>()
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        if !stack.is_empty() {
            result.insert(stack, AllocationStats { size, count });
        }
    }

    result
}

fn validate_file_for_reading(path: &str) -> std::io::Result<()> {
    let meta = fs::metadata(path)?;
    if !meta.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} is not a regular file", path),
        ));
    }
    let max_size = 256 * 1024 * 1024; // 256 MB
    if meta.len() > max_size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{} exceeds maximum allowed size (256 MB)", path),
        ));
    }
    Ok(())
}

pub fn diff_snapshots(path1: &str, path2: &str) {
    if let Err(e) = validate_file_for_reading(path1) {
        eprintln!("Error reading {}: {}", path1, e);
        return;
    }
    if let Err(e) = validate_file_for_reading(path2) {
        eprintln!("Error reading {}: {}", path2, e);
        return;
    }

    let file1_content = match fs::read_to_string(path1) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {}: {}", path1, e);
            return;
        }
    };
    let file2_content = match fs::read_to_string(path2) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {}: {}", path2, e);
            return;
        }
    };

    // Try both formats, whichever yields elements
    let mut snap1 = parse_json_map(&file1_content);
    if snap1.is_empty() {
        snap1 = parse_json_array(&file1_content);
    }

    let mut snap2 = parse_json_map(&file2_content);
    if snap2.is_empty() {
        snap2 = parse_json_array(&file2_content);
    }

    let mut net_differences = Vec::new();
    let mut new_paths = Vec::new();
    let mut freed_paths = Vec::new();

    for (stack, stats1) in &snap1 {
        if let Some(stats2) = snap2.get(stack) {
            let size_diff = stats2.size as isize - stats1.size as isize;
            let count_diff = stats2.count as isize - stats1.count as isize;
            if size_diff != 0 || count_diff != 0 {
                net_differences.push((stack.clone(), size_diff, count_diff));
            }
        } else {
            freed_paths.push((stack.clone(), stats1.size, stats1.count));
        }
    }

    for (stack, stats2) in &snap2 {
        if !snap1.contains_key(stack) {
            new_paths.push((stack.clone(), stats2.size, stats2.count));
        }
    }

    println!("=== Heap Profile Snapshot Diff ===");
    println!();

    println!("--- Net Differences (Changed Paths) ---");
    if net_differences.is_empty() {
        println!("  None");
    } else {
        net_differences.sort_by_key(|&(_, size_diff, _)| -size_diff.abs());
        for (stack, size_diff, count_diff) in net_differences {
            let size_sign = if size_diff >= 0 { "+" } else { "" };
            let count_sign = if count_diff >= 0 { "+" } else { "" };
            println!("  Stack: {}", stack);
            println!(
                "    Size Diff: {}{} bytes, Count Diff: {}{}",
                size_sign, size_diff, count_sign, count_diff
            );
        }
    }
    println!();

    println!("--- Newly Introduced Allocation Paths ---");
    if new_paths.is_empty() {
        println!("  None");
    } else {
        new_paths.sort_by_key(|&(_, size, _)| -(size as isize));
        for (stack, size, count) in new_paths {
            println!("  Stack: {}", stack);
            println!("    Size: +{} bytes, Count: +{}", size, count);
        }
    }
    println!();

    println!("--- Freed Allocation Paths ---");
    if freed_paths.is_empty() {
        println!("  None");
    } else {
        freed_paths.sort_by_key(|&(_, size, _)| -(size as isize));
        for (stack, size, count) in freed_paths {
            println!("  Stack: {}", stack);
            println!("    Size: -{} bytes, Count: -{}", size, count);
        }
    }
}
