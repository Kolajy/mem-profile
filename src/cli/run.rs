use num_format::{Locale, ToFormattedString};
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::io::{Read, Seek, SeekFrom};
use std::process::{exit, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[cfg(target_os = "macos")]
fn get_rss_bytes(
    pid: u32,
    _statm_path: &str,
    _page_size: u64,
    _statm_file: &mut Option<std::fs::File>,
) -> Option<u64> {
    if pid == 0 || pid > i32::MAX as u32 {
        return None;
    }

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
fn get_rss_bytes(
    _pid: u32,
    statm_path: &str,
    page_size: u64,
    statm_file: &mut Option<File>,
) -> Option<u64> {
    if statm_file.is_none() {
        if let Ok(f) = File::open(statm_path) {
            *statm_file = Some(f);
        } else {
            return None;
        }
    }

    if let Some(f) = statm_file {
        if f.seek(SeekFrom::Start(0)).is_err() {
            *statm_file = None;
            return None;
        }

        let mut buf = [0u8; 128];
        if let Ok(n) = f.read(&mut buf) {
            if let Ok(content) = std::str::from_utf8(&buf[..n]) {
                if let Some(resident_str) = content.split_whitespace().nth(1) {
                    if let Ok(resident) = resident_str.parse::<u64>() {
                        return Some(resident * page_size);
                    }
                }
            }
        } else {
            *statm_file = None;
        }
    }
    None
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn get_rss_bytes(
    _pid: u32,
    _statm_path: &str,
    _page_size: u64,
    _statm_file: &mut Option<std::fs::File>,
) -> Option<u64> {
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
        #[cfg(target_os = "linux")]
        let mut statm_file: Option<std::fs::File> = None;
        #[cfg(not(target_os = "linux"))]
        let mut statm_file: Option<std::fs::File> = None;

        loop {
            if let Ok(guard) = is_running_clone.lock() {
                if !*guard {
                    break;
                }
            } else {
                break;
            }
            if let Some(current_bytes) = get_rss_bytes(pid, &statm_path, page_size, &mut statm_file)
            {
                if let Ok(mut peak) = peak_rss_bytes_clone.lock() {
                    if current_bytes > *peak {
                        *peak = current_bytes;
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
            if let Ok(mut running) = is_running.lock() {
                *running = false;
            }
            let _ = poller_thread.join();
            exit(1);
        }
    };

    if let Ok(mut running) = is_running.lock() {
        *running = false;
    }
    let _ = poller_thread.join();

    let peak_rss_bytes_val = if let Ok(peak) = peak_rss_bytes.lock() {
        *peak
    } else {
        0u64
    };
    let peak_rss_mb = peak_rss_bytes_val as f64 / (1024.0 * 1024.0);
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

    eprintln!("\n=== Memory Profile ===");
    eprintln!("Command: {} {:?}", command, args);
    eprintln!(
        "Peak RSS: {} MB ({} bytes)",
        mb_str,
        peak_rss_bytes_val.to_formatted_string(&Locale::en)
    );

    if !status.success() {
        if let Some(code) = status.code() {
            exit(code);
        } else {
            exit(1);
        }
    }
}
