#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::io::Read;
use std::process::{exit, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[cfg(target_os = "macos")]
fn get_rss_bytes(pid: u32, _statm_path: &str, _page_size: u64) -> Option<u64> {
    let mut info: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
    let res = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTASKINFO,
            0,
            &mut info as *mut _ as *mut libc::c_void,
            std::mem::size_of::<libc::proc_taskinfo>() as i32,
        )
    };
    if res == std::mem::size_of::<libc::proc_taskinfo>() as i32 {
        return Some(info.pti_resident_size);
    }
    None
}

#[cfg(target_os = "linux")]
fn get_rss_bytes(_pid: u32, statm_path: &str, page_size: u64) -> Option<u64> {
    if let Ok(mut file) = File::open(statm_path) {
        let mut buf = [0u8; 128];
        if let Ok(n) = file.read(&mut buf) {
            if let Ok(content) = std::str::from_utf8(&buf[..n]) {
                if let Some(resident_str) = content.split_whitespace().nth(1) {
                    if let Ok(resident) = resident_str.parse::<u64>() {
                        return Some(resident * page_size);
                    }
                }
            }
        }
    }
    None
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn get_rss_bytes(_pid: u32, _statm_path: &str, _page_size: u64) -> Option<u64> {
    None
}

pub fn execute(command: String, args: Vec<String>) {
    let mut child = Command::new(&command)
        .args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|err| {
            eprintln!("Failed to execute '{}': {}", command, err);
            exit(1);
        });

    let pid = child.id();

    let peak_rss_bytes = Arc::new(Mutex::new(0u64));
    let peak_rss_bytes_clone = Arc::clone(&peak_rss_bytes);

    let is_running = Arc::new(Mutex::new(true));
    let is_running_clone = Arc::clone(&is_running);

    let poller_thread = thread::spawn(move || {
        let statm_path = format!("/proc/{}/statm", pid);
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;

        while *is_running_clone.lock().unwrap() {
            if let Some(current_bytes) = get_rss_bytes(pid, &statm_path, page_size) {
                let mut peak = peak_rss_bytes_clone.lock().unwrap();
                if current_bytes > *peak {
                    *peak = current_bytes;
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

    let peak_rss_bytes_val = *peak_rss_bytes.lock().unwrap();
    let peak_rss_mb = peak_rss_bytes_val as f64 / (1024.0 * 1024.0);

    eprintln!("\n=== Memory Profile ===");
    eprintln!("Command: {} {:?}", command, args);
    eprintln!(
        "Peak RSS: {:.2} MB ({} bytes)",
        peak_rss_mb, peak_rss_bytes_val
    );

    if !status.success() {
        if let Some(code) = status.code() {
            exit(code);
        } else {
            exit(1);
        }
    }
}
