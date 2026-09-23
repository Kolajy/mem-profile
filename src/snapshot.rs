use crate::allocator::REGISTRY;
use crate::backtrace::symbolicate_frames;
use num_format::{Buffer, Locale};
use std::fs::OpenOptions;
use std::io::{BufWriter, Write as _};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

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

    thread::spawn(move || loop {
        if SNAPSHOT_SIGUSR1.swap(false, Ordering::SeqCst) {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let name = format!("snapshot_sigusr1_{}.txt", timestamp);
            dump_to_file(Path::new(&name));
        }
        if SNAPSHOT_SIGUSR2.swap(false, Ordering::SeqCst) {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let name = format!("snapshot_sigusr2_{}.txt", timestamp);
            dump_to_file(Path::new(&name));
        }
        thread::sleep(Duration::from_millis(100));
    });
}

pub fn dump_to_file(path: &Path) {
    let mut grouped_allocations: rustc_hash::FxHashMap<Vec<*mut std::ffi::c_void>, (usize, usize)> =
        rustc_hash::FxHashMap::default();
    let mut total_bytes = 0;
    let mut total_alloc_count = 0;

    crate::allocator::IN_ALLOCATOR.with(|in_alloc| {
        let was_in = in_alloc.get();
        in_alloc.set(true);

        for shard_mutex in REGISTRY.get_shards() {
            if let Ok(shard) = shard_mutex.lock() {
                for (_, meta) in shard.iter() {
                    total_bytes += meta.size;
                    total_alloc_count += 1;
                    if let Some(entry) = grouped_allocations.get_mut(&meta.backtrace) {
                        entry.0 += meta.size;
                        entry.1 += 1;
                    } else {
                        grouped_allocations.insert(meta.backtrace.clone(), (meta.size, 1));
                    }
                }
            }
        }

        in_alloc.set(was_in);
    });

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_path = path.with_extension(format!("tmp.{}", timestamp));

    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    options.mode(0o600).custom_flags(libc::O_NOFOLLOW); // 🛡️ Sentinel: Secure file permissions to prevent info disclosure and symlink attacks

    let file = match options.open(&tmp_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create temporary snapshot file: {}", e);
            return;
        }
    };
    let mut buf_writer = BufWriter::new(file);

    let mut buf_allocs = Buffer::default();
    buf_allocs.write_formatted(&total_alloc_count, &Locale::en);
    let mut buf_bytes = Buffer::default();
    buf_bytes.write_formatted(&total_bytes, &Locale::en);

    let _ = writeln!(buf_writer, "Memory Snapshot");
    let _ = writeln!(buf_writer, "Total Allocations: {}", buf_allocs.as_str());
    let _ = writeln!(buf_writer, "Total Bytes: {}", buf_bytes.as_str());

    // Bolt: Grouping allocations by unique backtraces avoids O(N) backtrace cloning and implicitly avoids expensive repeated symbolication.
    let mut sorted_allocations: Vec<_> = grouped_allocations.into_iter().collect();
    sorted_allocations.sort_unstable_by_key(|&(_, (size, _))| std::cmp::Reverse(size));

    for (i, (frames, (size, count))) in sorted_allocations.into_iter().enumerate() {
        let mut i_buf = Buffer::default();
        i_buf.write_formatted(&(i + 1), &Locale::en);
        let mut size_buf = Buffer::default();
        size_buf.write_formatted(&size, &Locale::en);
        let mut count_buf = Buffer::default();
        count_buf.write_formatted(&count, &Locale::en);

        let _ = writeln!(
            buf_writer,
            "\nAllocation Group {}: {} bytes ({} allocations)",
            i_buf.as_str(),
            size_buf.as_str(),
            count_buf.as_str()
        );

        let symbols = symbolicate_frames(&frames);
        if symbols.is_empty() {
            let _ = writeln!(buf_writer, "  <no backtrace captured>");
        } else {
            for (idx, sym) in symbols.iter().enumerate() {
                let name = sym.name.as_deref().unwrap_or("<unknown>");
                if name.contains("mem_profile::") || name.contains("backtrace::") {
                    continue;
                }
                let _ = writeln!(buf_writer, "    #{}: {}", idx, sym);
            }
        }
    }

    drop(buf_writer);

    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        eprintln!("Failed to atomically rename snapshot file: {}", e);
    }
}
