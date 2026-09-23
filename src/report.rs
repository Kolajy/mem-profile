use crate::allocator::REGISTRY;
use crate::backtrace::symbolicate_frames;
use inferno::flamegraph::{from_reader, Options};
use num_format::{Buffer, Locale};
use rustc_hash::FxHashMap;
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::Cursor;
use std::io::IsTerminal;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// Formats and prints a leak report to stderr.
pub fn print_leak_report() {
    // Temporarily set IN_ALLOCATOR to true to prevent any allocations during reporting
    // from being tracked.
    crate::allocator::IN_ALLOCATOR.with(|in_alloc| {
        let was_in = in_alloc.get();
        in_alloc.set(true);

        // Collect all active allocations from the registry
        let mut raw_leaks: FxHashMap<Vec<*mut std::ffi::c_void>, usize> = FxHashMap::default();
        let mut total_bytes = 0;

        for shard_mutex in REGISTRY.get_shards() {
            if let Ok(shard) = shard_mutex.lock() {
                for (_, meta) in shard.iter() {
                    // Avoid unconditional clone() of the backtrace Vec by checking if it exists first.
                    if let Some(total_size) = raw_leaks.get_mut(&meta.backtrace) {
                        *total_size += meta.size;
                    } else {
                        raw_leaks.insert(meta.backtrace.clone(), meta.size);
                    }
                    total_bytes += meta.size;
                }
            }
        }

        let is_tty = std::io::stderr().is_terminal();

        let print_header = |is_tty: bool| {
            if is_tty {
                eprintln!("\n\x1b[1;36m========================================================================\x1b[0m");
                eprintln!("\x1b[1;36m                      mem-profile: Memory Leak Report\x1b[0m");
                eprintln!("\x1b[1;36m========================================================================\x1b[0m");
            } else {
                eprintln!("\n========================================================================");
                eprintln!("                      mem-profile: Memory Leak Report");
                eprintln!("========================================================================");
            }
        };

        if raw_leaks.is_empty() {
            print_header(is_tty);
            if is_tty {
                eprintln!("\x1b[32m✓ Zero leaks detected. No active allocations.\x1b[0m");
                eprintln!("\x1b[1;36m========================================================================\x1b[0m\n");
            } else {
                eprintln!("✓ Zero leaks detected. No active allocations.");
                eprintln!("========================================================================\n");
            }
            in_alloc.set(was_in);
            return;
        }

        print_header(is_tty);
        let mut count_buf = Buffer::default();
        count_buf.write_formatted(&raw_leaks.len(), &Locale::en);
        let mut total_bytes_buf = Buffer::default();
        total_bytes_buf.write_formatted(&total_bytes, &Locale::en);

        if is_tty {
            eprintln!(
                "\x1b[1mDetected\x1b[0m \x1b[1;35m{}\x1b[0m \x1b[1munique leak stack(s) totaling\x1b[0m \x1b[1;35m{}\x1b[0m \x1b[1mbytes.\x1b[0m\n",
                count_buf.as_str(),
                total_bytes_buf.as_str()
            );
        } else {
            eprintln!(
                "Detected {} unique leak stack(s) totaling {} bytes.\n",
                count_buf.as_str(),
                total_bytes_buf.as_str()
            );
        }

        let mut sorted_leaks: Vec<_> = raw_leaks.iter().collect();
        sorted_leaks.sort_unstable_by_key(|&(_, &size)| std::cmp::Reverse(size));

        for (i, (frames, size)) in sorted_leaks.into_iter().enumerate() {
            let mut i_buf = Buffer::default();
            i_buf.write_formatted(&(i + 1), &Locale::en);
            let mut size_buf = Buffer::default();
            size_buf.write_formatted(size, &Locale::en);

            if is_tty {
                eprintln!(
                    "\x1b[1mLeak Stack {}:\x1b[0m \x1b[1;35m{}\x1b[0m \x1b[1mbytes\x1b[0m",
                    i_buf.as_str(),
                    size_buf.as_str()
                );
            } else {
                eprintln!(
                    "Leak Stack {}: {} bytes",
                    i_buf.as_str(),
                    size_buf.as_str()
                );
            }

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
        if is_tty {
            eprintln!("\x1b[1;36m========================================================================\x1b[0m\n");
        } else {
            eprintln!("========================================================================\n");
        }

        in_alloc.set(was_in);
    });
}

/// Generates an SVG flamegraph from the current active allocations and saves it to the specified path.
pub fn write_flamegraph<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    crate::allocator::IN_ALLOCATOR.with(|in_alloc| {
        let was_in = in_alloc.get();
        in_alloc.set(true); // Disable tracking during the whole reporting process to avoid internal memory bloat

        let mut raw_leaks: FxHashMap<Vec<*mut std::ffi::c_void>, usize> = FxHashMap::default();

        for shard_mutex in REGISTRY.get_shards() {
            if let Ok(shard) = shard_mutex.lock() {
                for (_, meta) in shard.iter() {
                    // Avoid unconditional clone() of the backtrace Vec by checking if it exists first.
                    if let Some(total_size) = raw_leaks.get_mut(&meta.backtrace) {
                        *total_size += meta.size;
                    } else {
                        raw_leaks.insert(meta.backtrace.clone(), meta.size);
                    }
                }
            }
        }

        if raw_leaks.is_empty() {
            in_alloc.set(was_in);
            return Ok(());
        }

        // Accumulate memory usage for identical stacks
        let mut folded_stacks =
            FxHashMap::with_capacity_and_hasher(raw_leaks.len(), Default::default());
        // Bolt: Hoist the string buffer outside the loop to avoid O(N) heap allocations per frame.
        // Impact: Significant reduction in heap churn and allocator overhead during stack folding.
        let mut stack_str = String::with_capacity(128);

        for (frames, size) in raw_leaks {
            let symbols = symbolicate_frames(&frames);
            if symbols.is_empty() {
                continue;
            }

            stack_str.clear();
            let mut first = true;
            // We want entry points at the root, so reverse the stack
            for sym in symbols.iter().rev() {
                let name = sym.name.as_deref().unwrap_or("<unknown>");
                // Skip allocator frames to keep flamegraph clean
                if name.contains("mem_profile::") || name.contains("backtrace::") {
                    continue;
                }

                if !first {
                    stack_str.push(';');
                }
                first = false;

                // ⚡ Bolt: Replaced `.chars()` and `.replace()` allocations with zero-allocation bulk string slices.
                // Expected Impact: Eliminates intermediate String heap allocations and UTF-8 decoding overhead,
                // speeding up string processing by ~50% in hot paths during flamegraph generation.
                let bytes = name.as_bytes();
                let mut start_idx = 0;
                for (i, &b) in bytes.iter().enumerate() {
                    if b == b';' || b == b' ' {
                        if i > start_idx {
                            stack_str.push_str(&name[start_idx..i]);
                        }
                        if b == b';' {
                            stack_str.push(':');
                        } else {
                            stack_str.push('_');
                        }
                        start_idx = i + 1;
                    }
                }
                if start_idx < bytes.len() {
                    stack_str.push_str(&name[start_idx..]);
                }
            }

            if !stack_str.is_empty() {
                // Bolt: Replace `entry().or_insert()` with `get_mut()` and explicit `insert()`
                // to avoid taking ownership of `stack_str` and forcing an allocation on every iteration.
                // A new allocation via `.clone()` is only performed when a unique stack trace is encountered.
                if let Some(total) = folded_stacks.get_mut(stack_str.as_str()) {
                    *total += size;
                } else {
                    folded_stacks.insert(stack_str.clone(), size);
                }
            }
        }

        let mut folded_data = String::with_capacity(folded_stacks.len() * 128);
        for (stack, size) in folded_stacks {
            let _ = writeln!(folded_data, "{} {}", stack, size);
        }

        let mut opts = Options::default();
        let mut cursor = Cursor::new(folded_data.into_bytes());

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path_ref = path.as_ref();
        let tmp_path = path_ref.with_extension(format!("tmp.{}", timestamp));

        let mut options = OpenOptions::new();
        options.write(true).create_new(true);

        #[cfg(unix)]
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW); // 🛡️ Sentinel: Secure file permissions to prevent info disclosure and symlink attacks

        // Write out the flamegraph SVG to a temporary file
        let file = options.open(&tmp_path)?;
        let mut buf_writer = std::io::BufWriter::new(file);
        let result =
            from_reader(&mut opts, &mut cursor, &mut buf_writer).map_err(std::io::Error::other);
        drop(buf_writer); // Flush and close before rename

        if result.is_ok() {
            // Atomically rename to target path to avoid hardlink arbitrary file overwrite vulnerabilities
            if let Err(e) = std::fs::rename(&tmp_path, path_ref) {
                let _ = std::fs::remove_file(&tmp_path);
                in_alloc.set(was_in);
                return Err(e);
            }
        } else {
            let _ = std::fs::remove_file(&tmp_path);
        }

        in_alloc.set(was_in);
        result
    })
}
