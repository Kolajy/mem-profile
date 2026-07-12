use crate::allocator::REGISTRY;
use crate::backtrace::symbolicate_frames;
use std::collections::HashMap;

/// Exports active memory allocations to a "folded stack" format.
/// This format represents call stacks as semicolon-separated function names
/// followed by the number of bytes allocated by that stack.
/// Example: `main;foo;bar 1024`
pub fn export_folded_stacks() -> String {
    let mut stacks: HashMap<String, usize> = HashMap::new();

    crate::allocator::IN_ALLOCATOR.with(|in_alloc| {
        let was_in = in_alloc.get();
        in_alloc.set(true);

        for shard_mutex in REGISTRY.get_shards() {
            if let Ok(shard) = shard_mutex.lock() {
                for (_, meta) in shard.iter() {
                    let symbols = symbolicate_frames(&meta.backtrace);
                    let mut stack_frames = Vec::new();

                    // If we have no symbols (e.g. backtrace feature disabled),
                    // we'll just group everything under an unknown root.
                    if symbols.is_empty() {
                        stack_frames.push("<unknown>".to_string());
                    } else {
                        // Reverse the frames to put the root (e.g. main) first,
                        // and leaf (e.g. alloc) last.
                        for sym in symbols.iter().rev() {
                            let name = sym.name.as_deref().unwrap_or("<unknown>");

                            // Filter out internal mem-profile functions
                            if name.contains("mem_profile::") || name.contains("backtrace::") {
                                continue;
                            }

                            // Folded stacks use semicolons as frame separators.
                            // Ensure we don't have stray semicolons in function names.
                            let clean_name = name.replace(";", ",");
                            stack_frames.push(clean_name);
                        }
                    }

                    if stack_frames.is_empty() {
                        stack_frames.push("<unknown>".to_string());
                    }

                    let stack_str = stack_frames.join(";");
                    *stacks.entry(stack_str).or_insert(0) += meta.size;
                }
            }
        }

        in_alloc.set(was_in);
    });

    let mut output = String::new();
    // Sort keys to have deterministic output (useful for testing and diffing)
    let mut sorted_keys: Vec<_> = stacks.keys().collect();
    sorted_keys.sort();

    for key in sorted_keys {
        if let Some(size) = stacks.get(key) {
            output.push_str(&format!("{} {}\n", key, size));
        }
    }

    output
}
