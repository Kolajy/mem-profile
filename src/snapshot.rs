use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use crate::allocator::REGISTRY;
use crate::backtrace::symbolicate_frames;

static SNAPSHOT_SIGUSR1: AtomicBool = AtomicBool::new(false);
static SNAPSHOT_SIGUSR2: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigusr1(_sig: libc::c_int) {
    SNAPSHOT_SIGUSR1.store(true, Ordering::SeqCst);
}

extern "C" fn handle_sigusr2(_sig: libc::c_int) {
    SNAPSHOT_SIGUSR2.store(true, Ordering::SeqCst);
}

pub fn setup_signal_handlers() {
    unsafe {
        libc::signal(libc::SIGUSR1, handle_sigusr1 as *const () as usize);
        libc::signal(libc::SIGUSR2, handle_sigusr2 as *const () as usize);
    }

    thread::spawn(move || {
        loop {
            if SNAPSHOT_SIGUSR1.swap(false, Ordering::SeqCst) {
                dump_to_file(Path::new("snapshot_sigusr1.txt"));
            }
            if SNAPSHOT_SIGUSR2.swap(false, Ordering::SeqCst) {
                dump_to_file(Path::new("snapshot_sigusr2.txt"));
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
}

pub fn dump_to_file(path: &Path) {
    let mut allocations = Vec::new();
    let mut total_bytes = 0;

    crate::allocator::IN_ALLOCATOR.with(|in_alloc| {
        let was_in = in_alloc.get();
        in_alloc.set(true);

        for shard_mutex in REGISTRY.get_shards() {
            if let Ok(shard) = shard_mutex.lock() {
                for (ptr, meta) in shard.iter() {
                    allocations.push((*ptr, meta.size, meta.backtrace.clone()));
                    total_bytes += meta.size;
                }
            }
        }

        in_alloc.set(was_in);
    });

    let mut file = match File::create(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create snapshot file: {}", e);
            return;
        }
    };

    let _ = writeln!(file, "Memory Snapshot");
    let _ = writeln!(file, "Total Allocations: {}", allocations.len());
    let _ = writeln!(file, "Total Bytes: {}", total_bytes);

    for (i, (_ptr, size, frames)) in allocations.iter().enumerate() {
        let _ = writeln!(file, "\nAllocation {}: {} bytes", i + 1, size);
        let symbols = symbolicate_frames(frames);
        if symbols.is_empty() {
            let _ = writeln!(file, "  <no backtrace captured>");
        } else {
            for (idx, sym) in symbols.iter().enumerate() {
                let name = sym.name.as_deref().unwrap_or("<unknown>");
                if name.contains("mem_profile::") || name.contains("backtrace::") {
                    continue;
                }
                let _ = writeln!(file, "    #{}: {}", idx, sym);
            }
        }
    }
}
