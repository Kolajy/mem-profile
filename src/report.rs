use crate::allocator::REGISTRY;
use crate::backtrace::symbolicate_frames;
use inferno::flamegraph::{from_reader, Options};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Cursor;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// Formats and prints a leak report to stderr.
pub fn print_leak_report() {
    // Collect all active allocations from the registry
    let mut leaks = Vec::new();
    let mut total_bytes = 0;

    // Temporarily set IN_ALLOCATOR to true to prevent any allocations during reporting
    // from being tracked.
    crate::allocator::IN_ALLOCATOR.with(|in_alloc| {
        let was_in = in_alloc.get();
        in_alloc.set(true);

        for shard_mutex in REGISTRY.get_shards() {
            if let Ok(shard) = shard_mutex.lock() {
                for (ptr, meta) in shard.iter() {
                    leaks.push((*ptr, meta.size, meta.backtrace.clone()));
                    total_bytes += meta.size;
                }
            }
        }

        in_alloc.set(was_in);
    });

    if leaks.is_empty() {
        eprintln!("\n========================================================================");
        eprintln!("                      mem-profile: Memory Leak Report");
        eprintln!("========================================================================");
        eprintln!("No memory leaks detected!");
        eprintln!("========================================================================\n");
        return;
    }

    eprintln!("\n========================================================================");
    eprintln!("                      mem-profile: Memory Leak Report");
    eprintln!("========================================================================");
    eprintln!(
        "Detected {} leak(s) totaling {} bytes.\n",
        leaks.len(),
        total_bytes
    );

    for (i, (_ptr, size, frames)) in leaks.iter().enumerate() {
        eprintln!("Leak {}: {} bytes", i + 1, size);

        let symbols = symbolicate_frames(frames);
        if symbols.is_empty() {
            eprintln!("  <no backtrace captured>");
        } else {
            for (idx, sym) in symbols.iter().enumerate() {
                // Filter out internal mem-profile functions from the display if needed
                let name = sym.name.as_deref().unwrap_or("<unknown>");
                if name.contains("mem_profile::") || name.contains("backtrace::") {
                    continue;
                }
                eprintln!("    #{idx}: {sym}");
            }
        }
        eprintln!();
    }
    eprintln!("========================================================================\n");
}

/// Generates an SVG flamegraph from the current active allocations and saves it to the specified path.
pub fn write_flamegraph<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    crate::allocator::IN_ALLOCATOR.with(|in_alloc| {
        let was_in = in_alloc.get();
        in_alloc.set(true); // Disable tracking during the whole reporting process to avoid internal memory bloat

        let mut leaks = Vec::new();

        for shard_mutex in REGISTRY.get_shards() {
            if let Ok(shard) = shard_mutex.lock() {
                for (ptr, meta) in shard.iter() {
                    leaks.push((*ptr, meta.size, meta.backtrace.clone()));
                }
            }
        }

        if leaks.is_empty() {
            in_alloc.set(was_in);
            return Ok(());
        }

        // Accumulate memory usage for identical stacks
        let mut folded_stacks = HashMap::new();

        for (_ptr, size, frames) in leaks {
            let symbols = symbolicate_frames(&frames);
            if symbols.is_empty() {
                continue;
            }

            let mut stack = Vec::new();
            // We want entry points at the root, so reverse the stack
            for sym in symbols.iter().rev() {
                let name = sym.name.as_deref().unwrap_or("<unknown>");
                // Skip allocator frames to keep flamegraph clean
                if name.contains("mem_profile::") || name.contains("backtrace::") {
                    continue;
                }
                // Replace semicolons with colons to avoid inferno format conflicts
                let clean_name = name.replace(";", ":").replace(" ", "_");
                stack.push(clean_name);
            }

            if !stack.is_empty() {
                let stack_str = stack.join(";");
                *folded_stacks.entry(stack_str).or_insert(0) += size;
            }
        }

        let mut folded_data = String::new();
        for (stack, size) in folded_stacks {
            folded_data.push_str(&format!("{} {}\n", stack, size));
        }

        let mut opts = Options::default();
        let mut cursor = Cursor::new(folded_data.into_bytes());

        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);

        #[cfg(unix)]
        options.mode(0o600); // 🛡️ Sentinel: Secure file permissions to prevent info disclosure

        // Write out the flamegraph SVG
        let file = options.open(path)?;
        let result = from_reader(&mut opts, &mut cursor, file)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));

        in_alloc.set(was_in);
        result
    })
}
