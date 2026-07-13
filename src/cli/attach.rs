use std::fs;
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

        if let Ok(content) = fs::read_to_string(&statm_path) {
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(resident) = parts[1].parse::<u64>() {
                    if resident > peak_rss_pages {
                        peak_rss_pages = resident;
                    }
                }
            }
        } else {
            // Process might have exited between the kill check and read_to_string
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
