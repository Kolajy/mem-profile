use crate::allocator::REGISTRY;
use crate::backtrace::symbolicate_frames;
use std::collections::HashMap;
use std::fmt::Write as _;

/// Exports active memory allocations to a "folded stack" format.
/// This format represents call stacks as semicolon-separated function names
/// followed by the number of bytes allocated by that stack.
/// Example: `main;foo;bar 1024`
pub fn export_folded_stacks() -> String {
    let mut stacks: HashMap<String, usize> = HashMap::new();

    crate::allocator::IN_ALLOCATOR.with(|in_alloc| {
        let was_in = in_alloc.get();
        in_alloc.set(true);

        let mut raw_stacks: HashMap<Vec<*mut std::ffi::c_void>, usize> = HashMap::new();

        for shard_mutex in REGISTRY.get_shards() {
            if let Ok(shard) = shard_mutex.lock() {
                for (_, meta) in shard.iter() {
                    // Avoid unconditional clone() of the backtrace Vec by checking if it exists first.
                    if let Some(total_size) = raw_stacks.get_mut(&meta.backtrace) {
                        *total_size += meta.size;
                    } else {
                        raw_stacks.insert(meta.backtrace.clone(), meta.size);
                    }
                }
            }
        }

        for (backtrace, total_size) in raw_stacks {
            let symbols = symbolicate_frames(&backtrace);
            let mut stack_str = String::with_capacity(128);

            // If we have no symbols (e.g. backtrace feature disabled),
            // we'll just group everything under an unknown root.
            if symbols.is_empty() {
                stack_str.push_str("<unknown>");
            } else {
                // Reverse the frames to put the root (e.g. main) first,
                // and leaf (e.g. alloc) last.
                let mut first = true;
                for sym in symbols.iter().rev() {
                    let name = sym.name.as_deref().unwrap_or("<unknown>");

                    // Filter out internal mem-profile functions
                    if name.contains("mem_profile::") || name.contains("backtrace::") {
                        continue;
                    }

                    if !first {
                        stack_str.push(';');
                    }
                    first = false;

                    // Folded stacks use semicolons as frame separators.
                    // Ensure we don't have stray semicolons in function names.
                    for c in name.chars() {
                        if c == ';' {
                            stack_str.push(',');
                        } else {
                            stack_str.push(c);
                        }
                    }
                }
            }

            if stack_str.is_empty() {
                stack_str.push_str("<unknown>");
            }

            *stacks.entry(stack_str).or_insert(0) += total_size;
        }

        in_alloc.set(was_in);
    });

    let mut output = String::new();
    // Sort keys to have deterministic output (useful for testing and diffing)
    let mut sorted_keys: Vec<_> = stacks.keys().collect();
    sorted_keys.sort();

    for key in sorted_keys {
        if let Some(size) = stacks.get(key) {
            let _ = writeln!(output, "{} {}", key, size);
        }
    }

    output
}
