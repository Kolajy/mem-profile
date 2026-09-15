use num_format::{Locale, ToFormattedString};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub fn execute(pid: u32) {
    // 🛡️ Sentinel: Validate PID to prevent negative i32 wrapping which targets process groups or all processes
    if pid == 0 || pid > i32::MAX as u32 {
        eprintln!(
            "Error: Invalid PID {}. Must be a valid positive process ID.",
            pid
        );
        exit(1);
    }

    // Check if process exists and we have permission to signal it
    let alive = unsafe { libc::kill(pid as i32, 0) };
    if alive != 0 {
        eprintln!(
            "Error: Cannot attach to PID {}. Process might not exist or permission denied.",
            pid
        );
        exit(1);
    }

    println!("Attaching to PID {}...", pid);

    let is_interrupted = Arc::new(AtomicBool::new(false));
    let is_interrupted_clone = is_interrupted.clone();

    if let Err(e) = ctrlc::set_handler(move || {
        is_interrupted_clone.store(true, Ordering::SeqCst);
    }) {
        eprintln!("Error setting Ctrl-C handler: {}", e);
        exit(1);
    }

    let statm_path = format!("/proc/{}/statm", pid);
    let mut peak_rss_pages = 0u64;
    let mut statm_file: Option<File> = None;

    while !is_interrupted.load(Ordering::SeqCst) {
        // Check if process is still alive
        if unsafe { libc::kill(pid as i32, 0) } != 0 {
            println!("Process {} exited.", pid);
            break;
        }

        // ⚡ Bolt: Cache open file descriptors to significantly reduce syscall overhead
        // during high-frequency polling loops by bypassing open/close on every tick.
        if statm_file.is_none() {
            if let Ok(f) = File::open(&statm_path) {
                statm_file = Some(f);
            }
        }

        if let Some(file) = &mut statm_file {
            // ⚡ Bolt: Rewind the cursor instead of reopening to read updated process metrics
            if file.seek(SeekFrom::Start(0)).is_err() {
                break;
            }

            let mut buf = [0u8; 128];
            if let Ok(n) = file.read(&mut buf) {
                // ⚡ Bolt: Parse the second integer (RSS pages) directly from the raw byte buffer.
                // This eliminates UTF-8 validation (`from_utf8`) and string splitting overhead in the high-frequency polling loop.
                let mut state = 0;
                let mut pages: u64 = 0;
                let mut found = false;
                for &b in &buf[..n] {
                    if b == b' ' {
                        if state == 0 {
                            state = 1;
                        } else if state == 1 {
                            break;
                        }
                    } else if state == 1 && b.is_ascii_digit() {
                        pages = pages * 10 + (b - b'0') as u64;
                        found = true;
                    }
                }
                if found && pages > peak_rss_pages {
                    peak_rss_pages = pages;
                }
            }
        } else {
            // Process might have exited between the kill check and file open
            break;
        }

        thread::sleep(Duration::from_millis(100));
    }

    if is_interrupted.load(Ordering::SeqCst) {
        println!("Monitoring interrupted by user.");
    }

    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    let peak_rss_bytes = peak_rss_pages * page_size;
    let peak_rss_mb = peak_rss_bytes as f64 / (1024.0 * 1024.0);
    let int_part = peak_rss_mb.trunc() as u64;
    let frac_part = (peak_rss_mb.fract() * 100.0).round() as u64;
    let (int_part, frac_part) = if frac_part == 100 {
        (int_part + 1, 0)
    } else {
        (int_part, frac_part)
    };
    let mb_str = format!(
        "{}.{:02}",
        int_part.to_formatted_string(&Locale::en),
        frac_part
    );

    use std::io::IsTerminal;
    let is_tty = std::io::stderr().is_terminal();

    if is_tty {
        eprintln!("\n\x1b[1;36m=== Memory Profile ===\x1b[0m");
        eprintln!("\x1b[1mPID:\x1b[0m {}", pid);
        eprintln!(
            "\x1b[1mPeak RSS:\x1b[0m \x1b[1;35m{} MB\x1b[0m ({} bytes)",
            mb_str,
            peak_rss_bytes.to_formatted_string(&Locale::en)
        );
    } else {
        eprintln!("\n=== Memory Profile ===");
        eprintln!("PID: {}", pid);
        eprintln!(
            "Peak RSS: {} MB ({} bytes)",
            mb_str,
            peak_rss_bytes.to_formatted_string(&Locale::en)
        );
    }
}
