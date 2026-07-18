use std::fs::File;
use std::io::Read;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub fn execute(pid: u32) {
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

    ctrlc::set_handler(move || {
        is_interrupted_clone.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let statm_path = format!("/proc/{}/statm", pid);
    let mut peak_rss_pages = 0u64;

    while !is_interrupted.load(Ordering::SeqCst) {
        // Check if process is still alive
        if unsafe { libc::kill(pid as i32, 0) } != 0 {
            println!("Process {} exited.", pid);
            break;
        }

        if let Ok(mut file) = File::open(&statm_path) {
            let mut buf = [0u8; 128];
            if let Ok(n) = file.read(&mut buf) {
                if let Ok(content) = std::str::from_utf8(&buf[..n]) {
                    if let Some(resident_str) = content.split_whitespace().nth(1) {
                        if let Ok(resident) = resident_str.parse::<u64>() {
                            if resident > peak_rss_pages {
                                peak_rss_pages = resident;
                            }
                        }
                    }
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

    eprintln!("\n=== Memory Profile ===");
    eprintln!("PID: {}", pid);
    eprintln!("Peak RSS: {:.2} MB ({} bytes)", peak_rss_mb, peak_rss_bytes);
}
