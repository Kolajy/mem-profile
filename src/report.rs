use crate::allocator::REGISTRY;
use crate::backtrace::symbolicate_frames;

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
