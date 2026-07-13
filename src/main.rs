use std::env;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn get_rss(statm_path: &str, page_size: u64) -> Option<u64> {
    if let Ok(contents) = fs::read_to_string(statm_path) {
        if let Some(part) = contents.split_whitespace().nth(1) {
            if let Ok(pages) = part.parse::<u64>() {
                return Some(pages * page_size);
            }
        }
    }
    None
}

fn format_bytes(v: f64) -> String {
    if v < 1024.0 {
        format!("{} B", v as u64)
    } else if v < 1024.0 * 1024.0 {
        format!("{:.1} KB", v / 1024.0)
    } else if v < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB", v / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", v / (1024.0 * 1024.0 * 1024.0))
    }
}

fn draw_graph(data: &[f64], total_duration: f64) {
    if data.is_empty() {
        println!("\nNo memory data collected (process ran too fast).");
        return;
    }

    let height = 15;
    let width = 60;

    let mut display_data = Vec::new();
    if data.len() > width {
        let chunk_size = data.len() as f64 / width as f64;
        for i in 0..width {
            let start = (i as f64 * chunk_size) as usize;
            let end = ((i + 1) as f64 * chunk_size) as usize;
            let end = if end > data.len() { data.len() } else { end };

            let max_val = data[start..end].iter().copied().fold(0.0_f64, f64::max);
            display_data.push(max_val);
        }
    } else {
        display_data = data.to_vec();
    }

    let min_v = display_data.iter().copied().fold(f64::INFINITY, f64::min);
    let mut max_v = display_data
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);

    if (min_v - max_v).abs() < f64::EPSILON {
        max_v += 1.0;
    }

    let range_v = max_v - min_v;

    let mut grid = vec![vec![' '; display_data.len()]; height];

    for (x, &val) in display_data.iter().enumerate() {
        let y = (((val - min_v) / range_v) * (height - 1) as f64).round() as usize;
        let y_idx = height - 1 - y;
        if y_idx < height {
            grid[y_idx][x] = '*';
        }
    }

    for x in 0..display_data.len() - 1 {
        let y1 = (((display_data[x] - min_v) / range_v) * (height - 1) as f64).round() as usize;
        let y2 = (((display_data[x + 1] - min_v) / range_v) * (height - 1) as f64).round() as usize;

        for y in (std::cmp::min(y1, y2) + 1)..std::cmp::max(y1, y2) {
            let y_idx = height - 1 - y;
            if y_idx < height {
                grid[y_idx][x] = '|';
            }
        }
    }

    println!("\n{}", "=".repeat(80));
    println!("Memory Usage (RSS) Timeline");
    println!("{}", "=".repeat(80));

    for r in 0..height {
        let val = max_v - (range_v * r as f64 / (height - 1) as f64);
        let label = format!("{:>10}", format_bytes(val));
        let row_str: String = grid[r].iter().collect();
        println!("{} | {}", label, row_str);
    }
    println!("{} +{}", " ".repeat(11), "-".repeat(display_data.len()));

    let end_label = format!("{:.1}s", total_duration);
    let mut spaces = display_data.len() as isize - 2 - end_label.len() as isize + 2;
    if spaces < 1 {
        spaces = 1;
    }
    println!(
        "{} 0s{}{}",
        " ".repeat(11),
        " ".repeat(spaces as usize),
        end_label
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: mem-profile <command> [args...]");
        std::process::exit(1);
    }

    let cmd_name = &args[1];
    let cmd_args = &args[2..];

    let mut child = match Command::new(cmd_name).args(cmd_args).spawn() {
        Ok(c) => c,
        Err(e) => {
            println!("Error executing command: {}", e);
            std::process::exit(1);
        }
    };

    let pid = child.id();
    let statm_path = format!("/proc/{}/statm", pid);
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 };
    let statm_path = format!("/proc/{}/statm", pid);

    let is_running = Arc::new(AtomicBool::new(true));
    let rss_data = Arc::new(Mutex::new(Vec::new()));

    let is_running_clone = is_running.clone();
    let rss_data_clone = rss_data.clone();

    let start_time = Instant::now();

    let monitor_thread = thread::spawn(move || {
        let mut sleep_time = 100;
        let limit = 8192;
        while is_running_clone.load(Ordering::Relaxed) {
            if let Some(rss) = get_rss(&statm_path, page_size) {
                if rss > 0 {
                    let mut data = rss_data_clone.lock().unwrap();
                    data.push(rss as f64);
                    if data.len() >= limit {
                        let mut new_data = Vec::with_capacity(limit / 2);
                        for chunk in data.chunks(2) {
                            if chunk.len() == 2 {
                                new_data.push(chunk[0].max(chunk[1]));
                            } else {
                                new_data.push(chunk[0]);
                            }
                        }
                        *data = new_data;
                        sleep_time *= 2;
                    }
                }
            }
            thread::sleep(Duration::from_millis(sleep_time));
        }
    });

    let is_interrupted = Arc::new(AtomicBool::new(false));
    let is_interrupted_clone = is_interrupted.clone();

    ctrlc::set_handler(move || {
        is_interrupted_clone.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let exit_routine = |child: &mut std::process::Child, exit_code: i32| -> ! {
        let _ = child.kill();
        is_running.store(false, Ordering::Relaxed);
        monitor_thread.join().unwrap();
        let total_duration = start_time.elapsed().as_secs_f64();
        let data = rss_data.lock().unwrap();
        draw_graph(&data, total_duration);
        std::process::exit(exit_code);
    };

    while !is_interrupted.load(Ordering::SeqCst) {
        if let Ok(Some(status)) = child.try_wait() {
            exit_routine(&mut child, status.code().unwrap_or(0));
        }
        thread::sleep(Duration::from_millis(10));
    }

    // If we reach here, it was interrupted
    exit_routine(&mut child, 130); // Standard exit code for SIGINT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        // Bytes
        assert_eq!(format_bytes(0.0), "0 B");
        assert_eq!(format_bytes(512.0), "512 B");
        assert_eq!(format_bytes(1023.0), "1023 B");

        // Kilobytes
        assert_eq!(format_bytes(1024.0), "1.0 KB");
        assert_eq!(format_bytes(1536.0), "1.5 KB");
        assert_eq!(format_bytes(1024.0 * 1024.0 - 1.0), "1024.0 KB");

        // Megabytes
        assert_eq!(format_bytes(1024.0 * 1024.0), "1.0 MB");
        assert_eq!(format_bytes(1.5 * 1024.0 * 1024.0), "1.5 MB");
        assert_eq!(format_bytes(1024.0 * 1024.0 * 1024.0 - 1.0), "1024.0 MB");

        // Gigabytes
        assert_eq!(format_bytes(1024.0 * 1024.0 * 1024.0), "1.0 GB");
        assert_eq!(format_bytes(1.5 * 1024.0 * 1024.0 * 1024.0), "1.5 GB");
        assert_eq!(format_bytes(10.0 * 1024.0 * 1024.0 * 1024.0), "10.0 GB");
    }
}
