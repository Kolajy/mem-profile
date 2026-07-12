use clap::Parser;
use std::fs;
use std::process::{exit, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(required = true)]
    command: String,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let mut child = Command::new(&args.command)
        .args(&args.args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|err| {
            eprintln!("Failed to execute '{}': {}", args.command, err);
            exit(1);
        });

    let pid = child.id();

    let peak_rss_pages = Arc::new(Mutex::new(0u64));
    let peak_rss_pages_clone = Arc::clone(&peak_rss_pages);

    let is_running = Arc::new(Mutex::new(true));
    let is_running_clone = Arc::clone(&is_running);

    let poller_thread = thread::spawn(move || {
        let statm_path = format!("/proc/{}/statm", pid);
        while *is_running_clone.lock().unwrap() {
            if let Ok(content) = fs::read_to_string(&statm_path) {
                // /proc/[pid]/statm fields:
                // size resident shared text lib data dt
                let parts: Vec<&str> = content.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(resident) = parts[1].parse::<u64>() {
                        let mut peak = peak_rss_pages_clone.lock().unwrap();
                        if resident > *peak {
                            *peak = resident;
                        }
                    }
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
    });

    let status = match child.wait() {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Failed to wait on child: {}", err);
            *is_running.lock().unwrap() = false;
            let _ = poller_thread.join();
            exit(1);
        }
    };

    *is_running.lock().unwrap() = false;
    let _ = poller_thread.join();

    let peak_pages = *peak_rss_pages.lock().unwrap();

    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    let peak_rss_bytes = peak_pages * page_size;
    let peak_rss_mb = peak_rss_bytes as f64 / (1024.0 * 1024.0);

    eprintln!("\n=== Memory Profile ===");
    eprintln!("Command: {} {:?}", args.command, args.args);
    eprintln!("Peak RSS: {:.2} MB ({} bytes)", peak_rss_mb, peak_rss_bytes);

    if !status.success() {
        if let Some(code) = status.code() {
            exit(code);
        } else {
            exit(1);
        }
    }
}
