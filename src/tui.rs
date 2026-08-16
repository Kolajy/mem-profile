use crate::allocator::REGISTRY;
use crate::backtrace::symbolicate_frames;
use crate::snapshot::dump_to_file;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use num_format::{Locale, ToFormattedString};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Cell, Chart, Dataset, GraphType, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState,
    },
    Frame, Terminal,
};
use std::{
    collections::{HashMap, VecDeque},
    io,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

pub fn run() {
    let pid = std::process::id();
    // Setup terminal
    if let Err(e) = enable_raw_mode() {
        eprintln!("Error: Failed to enable raw mode: {}", e);
        return;
    }
    let mut stdout = io::stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
        eprintln!("Error: Failed to execute terminal commands: {}", e);
        let _ = disable_raw_mode();
        return;
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error: Failed to create terminal: {}", e);
            let _ = disable_raw_mode();
            return;
        }
    };
    if let Err(e) = terminal.hide_cursor() {
        eprintln!("Error: Failed to hide cursor: {}", e);
        let _ = disable_raw_mode();
        return;
    }

    // Create app state
    let app = Arc::new(Mutex::new(App::new(pid)));
    let app_clone = Arc::clone(&app);

    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = Arc::clone(&is_running);

    let monitor_thread = thread::spawn(move || {
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 };
        let start_time = Instant::now();
        // Bolt: Pre-allocate the statm_path string to prevent dynamic allocation on every polling tick
        let statm_path = format!("/proc/{}/statm", pid);
        #[cfg(target_os = "linux")]
        let mut statm_file: Option<std::fs::File> = None;
        #[cfg(not(target_os = "linux"))]
        let mut statm_file: Option<std::fs::File> = None;

        while is_running_clone.load(Ordering::Relaxed) {
            let is_paused;
            {
                if let Ok(app) = app_clone.lock() {
                    is_paused = app.is_paused;
                } else {
                    break;
                }
            }

            if !is_paused {
                if let Some(rss_bytes) = get_rss_bytes(pid, page_size, &statm_path, &mut statm_file)
                {
                    if let Ok(mut app) = app_clone.lock() {
                        let elapsed = start_time.elapsed().as_secs_f64();
                        let rss_f64 = rss_bytes as f64;
                        app.rss_history.push_back((elapsed, rss_f64));
                        if rss_f64 > app.peak_rss {
                            app.peak_rss = rss_f64;
                        }
                        // keep only last N points to avoid unbounded growth
                        if app.rss_history.len() > 1000 {
                            app.rss_history.pop_front();
                        }
                    } else {
                        break;
                    }
                } else {
                    // Process died or can't read
                    if let Ok(mut app) = app_clone.lock() {
                        app.process_exited = true;
                    } else {
                        break;
                    }
                }
            }

            thread::sleep(Duration::from_millis(500));
        }
    });

    let res = run_app(&mut terminal, app, is_running.clone());

    // Restore terminal
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();

    is_running.store(false, Ordering::Relaxed);
    let _ = monitor_thread.join();

    if let Err(err) = res {
        println!("{:?}", err);
    }
}

#[allow(unused_variables)]
fn get_rss_bytes(
    pid: u32,
    page_size: u64,
    statm_path: &str,
    statm_file: &mut Option<std::fs::File>,
) -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        use std::io::{Read, Seek, SeekFrom};
        if statm_file.is_none() {
            if let Ok(f) = std::fs::File::open(statm_path) {
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
    }
    #[cfg(target_os = "macos")]
    {
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
    }
    None
}

struct App {
    #[allow(dead_code)]
    pid: u32,
    // Bolt: Pre-allocate TUI Title string to eliminate string formatting in UI render loop
    pid_title: String,
    rss_history: VecDeque<(f64, f64)>, // (time_s, bytes)
    peak_rss: f64,
    is_paused: bool,
    process_exited: bool,
    table_state: TableState,
    sort_by_size: bool, // true: size, false: count
    last_snapshot_time: Option<Instant>,
    last_snapshot_name: Option<String>,
    symbol_cache: HashMap<FramePtrs, Arc<String>>,
}

// Wrapping the raw pointer backtrace vectors to safely implement Send/Sync without risking
// future non-thread-safe fields in App being inadvertently allowed across threads.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct FramePtrs(Vec<*mut std::ffi::c_void>);

// Safe because the raw pointers are just instruction addresses, not owned memory.
unsafe impl Send for FramePtrs {}
unsafe impl Sync for FramePtrs {}

impl std::borrow::Borrow<[*mut std::ffi::c_void]> for FramePtrs {
    fn borrow(&self) -> &[*mut std::ffi::c_void] {
        &self.0
    }
}

impl App {
    fn new(pid: u32) -> Self {
        Self {
            pid,
            pid_title: format!(" Mem-Profile TUI | PID: {} ", pid),
            rss_history: VecDeque::new(),
            peak_rss: 0.0,
            is_paused: false,
            process_exited: false,
            table_state: TableState::default(),
            sort_by_size: true,
            last_snapshot_time: None,
            last_snapshot_name: None,
            symbol_cache: HashMap::new(),
        }
    }

    fn next(&mut self, items_len: usize) {
        if items_len == 0 {
            self.table_state.select(None);
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= items_len.saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn scroll_down(&mut self, items_len: usize) {
        if items_len == 0 {
            self.table_state.select(None);
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= items_len.saturating_sub(1) {
                    items_len.saturating_sub(1)
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn scroll_up(&mut self, items_len: usize) {
        if items_len == 0 {
            self.table_state.select(None);
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self, items_len: usize) {
        if items_len == 0 {
            self.table_state.select(None);
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    items_len.saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn page_up(&mut self, items_len: usize) {
        if items_len == 0 {
            self.table_state.select(None);
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => i.saturating_sub(10),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn page_down(&mut self, items_len: usize) {
        if items_len == 0 {
            self.table_state.select(None);
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i + 10 >= items_len {
                    items_len.saturating_sub(1)
                } else {
                    i + 10
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn first(&mut self, items_len: usize) {
        if items_len == 0 {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
    }

    fn last(&mut self, items_len: usize) {
        if items_len == 0 {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(items_len.saturating_sub(1)));
        }
    }
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: Arc<Mutex<App>>,
    _is_running: Arc<AtomicBool>,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    // Bolt: Hoist these temporary collections out of the render loop to prevent continuous heap allocation on every tick.
    let mut raw_allocs_cache: HashMap<Vec<*mut std::ffi::c_void>, (usize, usize)> = HashMap::new();
    let mut folded_cache: HashMap<Arc<String>, (usize, usize)> = HashMap::new();
    let mut items: Vec<(Arc<String>, usize, usize, String, String)> = Vec::with_capacity(128);

    loop {
        {
            if let Ok(mut app_lock) = app.lock() {
                get_active_allocations(
                    app_lock.sort_by_size,
                    &mut app_lock.symbol_cache,
                    &mut raw_allocs_cache,
                    &mut folded_cache,
                    &mut items,
                );

                terminal.draw(|f| ui(f, &mut app_lock, &items))?;
            } else {
                return Err(io::Error::other("App mutex poisoned"));
            }
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    let mut app_lock = match app.lock() {
                        Ok(lock) => lock,
                        Err(_) => return Err(io::Error::other("App mutex poisoned")),
                    };
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return Ok(());
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(());
                        }
                        KeyCode::Char('p') | KeyCode::Char(' ') => {
                            if !app_lock.process_exited {
                                app_lock.is_paused = !app_lock.is_paused;
                            }
                        }
                        KeyCode::Char('s') => {
                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_nanos();
                            let name = format!("tui_snapshot_{}.txt", timestamp);
                            dump_to_file(Path::new(&name));
                            app_lock.last_snapshot_time = Some(Instant::now());
                            app_lock.last_snapshot_name = Some(name);
                        }
                        KeyCode::Char('r') => {
                            app_lock.sort_by_size = !app_lock.sort_by_size;
                            if !items.is_empty() {
                                app_lock.table_state.select(Some(0));
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app_lock.next(items.len());
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app_lock.previous(items.len());
                        }
                        KeyCode::PageUp => {
                            app_lock.page_up(items.len());
                        }
                        KeyCode::PageDown => {
                            app_lock.page_down(items.len());
                        }
                        KeyCode::Home => {
                            app_lock.first(items.len());
                        }
                        KeyCode::End => {
                            app_lock.last(items.len());
                        }
                        _ => {}
                    }
                }
                Event::Mouse(mouse) => {
                    let mut app_lock = match app.lock() {
                        Ok(lock) => lock,
                        Err(_) => return Err(io::Error::other("App mutex poisoned")),
                    };
                    match mouse.kind {
                        event::MouseEventKind::ScrollDown => app_lock.scroll_down(items.len()),
                        event::MouseEventKind::ScrollUp => app_lock.scroll_up(items.len()),
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

// Returns a list of (backtrace_string, total_size, count)
fn get_active_allocations(
    sort_by_size: bool,
    symbol_cache: &mut HashMap<FramePtrs, Arc<String>>,
    raw_allocs: &mut HashMap<Vec<*mut std::ffi::c_void>, (usize, usize)>,
    folded: &mut HashMap<Arc<String>, (usize, usize)>,
    result: &mut Vec<(Arc<String>, usize, usize, String, String)>,
) {
    crate::allocator::IN_ALLOCATOR.with(|in_alloc| {
        let was_in = in_alloc.get();
        in_alloc.set(true);

        raw_allocs.clear();
        folded.clear();

        for shard_mutex in REGISTRY.get_shards() {
            if let Ok(shard) = shard_mutex.lock() {
                for (_, meta) in shard.iter() {
                    // Bolt: Avoid cloning `meta.backtrace` for already memoized backtraces.
                    if let Some(cached) = symbol_cache.get(meta.backtrace.as_slice()) {
                        if let Some(entry) = folded.get_mut(cached) {
                            entry.0 += meta.size;
                            entry.1 += 1;
                        } else {
                            folded.insert(Arc::clone(cached), (meta.size, 1));
                        }
                    } else {
                        if let Some(entry) = raw_allocs.get_mut(&meta.backtrace) {
                            entry.0 += meta.size;
                            entry.1 += 1;
                        } else {
                            raw_allocs.insert(meta.backtrace.clone(), (meta.size, 1));
                        }
                    }
                }
            }
        }
        for (frames, (total_size, count)) in raw_allocs.drain() {
            // Bolt: Symbolication is extremely expensive. Memoize the resolved string representation
            // of raw backtrace pointers to prevent severe CPU bottlenecks during the TUI render loop.
            // Bolt: Avoid unconditional clone() of `frames` here by moving ownership into `FramePtrs`.
            let frame_ptrs = FramePtrs(frames);
            if let Some(cached) = symbol_cache.get(&frame_ptrs) {
                if let Some(entry) = folded.get_mut(cached) {
                    entry.0 += total_size;
                    entry.1 += count;
                } else {
                    folded.insert(Arc::clone(cached), (total_size, count));
                }
            } else {
                let symbols = symbolicate_frames(&frame_ptrs.0);
                let mut s_buf = String::with_capacity(128);
                let mut first = true;
                for sym in symbols.iter() {
                    let name = sym.name.as_deref().unwrap_or("<unknown>");
                    if name.contains("mem_profile::") || name.contains("backtrace::") {
                        continue;
                    }
                    if !first {
                        s_buf.push_str(" <- ");
                    }
                    s_buf.push_str(name);
                    first = false;
                }
                if s_buf.is_empty() {
                    s_buf.push_str("<unknown>");
                }

                let s = Arc::new(s_buf);

                if let Some(entry) = folded.get_mut(&s) {
                    entry.0 += total_size;
                    entry.1 += count;
                } else {
                    folded.insert(Arc::clone(&s), (total_size, count));
                }
                symbol_cache.insert(frame_ptrs, Arc::clone(&s));
            }
        }

        use std::fmt::Write;
        let mut i = 0;
        for (k, v) in folded.iter() {
            if i < result.len() {
                result[i].0 = Arc::clone(k);
                result[i].1 = v.0;
                result[i].2 = v.1;
                result[i].3.clear();
                let _ = write_bytes(&mut result[i].3, v.0 as f64);
                result[i].4.clear();
                let mut buf = num_format::Buffer::default();
                buf.write_formatted(&v.1, &num_format::Locale::en);
                let _ = write!(result[i].4, "{}", buf.as_str());
            } else {
                let mut size_str = String::with_capacity(32);
                let _ = write_bytes(&mut size_str, v.0 as f64);
                let mut count_str = String::with_capacity(32);
                let mut buf = num_format::Buffer::default();
                buf.write_formatted(&v.1, &num_format::Locale::en);
                let _ = write!(count_str, "{}", buf.as_str());
                result.push((Arc::clone(k), v.0, v.1, size_str, count_str));
            }
            i += 1;
        }
        result.truncate(i);

        if sort_by_size {
            result.sort_by(|a, b| b.1.cmp(&a.1));
        } else {
            result.sort_by(|a, b| b.2.cmp(&a.2));
        }

        in_alloc.set(was_in);
    })
}

fn ui(f: &mut Frame, app: &mut App, items: &[(Arc<String>, usize, usize, String, String)]) {
    if items.is_empty() {
        app.table_state.select(None);
    } else {
        match app.table_state.selected() {
            None => app.table_state.select(Some(0)),
            Some(i) if i >= items.len() => app.table_state.select(Some(items.len() - 1)),
            _ => {}
        }
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ])
        .split(f.size());

    // Title / Status
    let (status_span, border_style) = if app.process_exited {
        (
            Span::styled(
                " [PROCESS EXITED] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Style::default().fg(Color::Red),
        )
    } else if app.is_paused {
        (
            Span::styled(
                " [PAUSED] ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Style::default().fg(Color::Yellow),
        )
    } else {
        (
            Span::styled(
                " [RUNNING] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Style::default().fg(Color::DarkGray),
        )
    };

    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let spans = vec![Span::raw(app.pid_title.as_str()), status_span];

    let show_flash = app
        .last_snapshot_time
        .is_some_and(|time| time.elapsed() < Duration::from_secs(3));

    let mut key_spans = vec![];
    if show_flash {
        let msg = if let Some(ref name) = app.last_snapshot_name {
            format!(" Snapshot saved to {}! ", name)
        } else {
            " Snapshot Saved! ".to_string()
        };
        key_spans.push(Span::styled(
            msg,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        key_spans.push(Span::raw(" Keys: "));
        if !app.process_exited {
            key_spans.push(Span::styled("[p/Space]", key_style));
            key_spans.push(Span::raw(if app.is_paused {
                " resume, "
            } else {
                " pause, "
            }));
        }
        key_spans.extend(vec![
            Span::styled("[s]", key_style),
            Span::raw("napshot, "),
            Span::styled("[r]", key_style),
            Span::raw("e-sort, "),
            Span::styled("[q]", key_style),
            Span::raw("uit, "),
            Span::styled("[↑/↓/j/k/Pg/Home/End]", key_style),
            Span::raw(" nav "),
        ]);
    }

    let title = Line::from(spans);

    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    let current_rss = if let Some(last) = app.rss_history.back() {
        format_bytes(last.1)
    } else {
        "N/A".to_string()
    };

    let peak_rss_str = if app.peak_rss > 0.0 {
        format_bytes(app.peak_rss)
    } else {
        "N/A".to_string()
    };

    let info_line = Line::from(vec![
        Span::raw(if app.process_exited {
            "Final RSS: "
        } else {
            "Current RSS: "
        }),
        Span::styled(
            current_rss,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | Peak RSS: "),
        Span::styled(
            peak_rss_str,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let info = ratatui::widgets::Paragraph::new(info_line)
        .block(title_block)
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(info, chunks[0]);

    // Graph
    if app.rss_history.is_empty() {
        let block = Block::default()
            .title("RSS Timeline (Last 60s)")
            .borders(Borders::ALL)
            .border_style(border_style);
        let msg = if app.process_exited {
            "No memory data collected.".to_string()
        } else {
            let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let idx = (t / 100) as usize % spinner.len();
            format!("{} Waiting for initial memory reading...", spinner[idx])
        };
        let info = ratatui::widgets::Paragraph::new(msg)
            .block(block)
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(info, chunks[1]);
    } else {
        // Zero-allocation: take a reference to the slice instead of unconditionally cloning the entire history Vec every frame.
        let data: &[(f64, f64)] = app.rss_history.make_contiguous();
        let max_time = data.last().map(|d| d.0).unwrap_or(10.0).max(10.0);
        let min_time = if max_time > 60.0 {
            max_time - 60.0
        } else {
            0.0
        };

        // Bolt: Filter data to only include points within the visible time window
        // to avoid having Ratatui's Chart widget process up to 1000 off-screen points,
        // and to ensure max_bytes accurately scales the Y-axis to the visible data.
        // Use saturating_sub(1) to keep one off-screen point for drawing the line entering the chart.
        let start_idx = data.partition_point(|d| d.0 < min_time).saturating_sub(1);
        let visible_data = &data[start_idx..];

        let max_bytes = visible_data
            .iter()
            .map(|d| d.1)
            .fold(0.0, f64::max)
            .max(1024.0 * 1024.0);

        let datasets = vec![Dataset::default()
            .name("RSS")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(visible_data)];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title("RSS Timeline (Last 60s)")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .x_axis(
                Axis::default()
                    .title("Time (s)")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([min_time, max_time])
                    .labels(vec![
                        Span::raw(format!("{:.1}", min_time)),
                        Span::raw(format!("{:.1}", (min_time + max_time) / 2.0)),
                        Span::raw(format!("{:.1}", max_time)),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .title("Bytes")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, max_bytes])
                    .labels(vec![
                        Span::raw("0 B"),
                        Span::raw(format_bytes(max_bytes / 2.0)),
                        Span::raw(format_bytes(max_bytes)),
                    ]),
            );

        f.render_widget(chart, chunks[1]);
    }

    // Table
    let (size_header, size_style) = if app.sort_by_size {
        (
            "Size ▼",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        ("Size  ", Style::default().fg(Color::Yellow))
    };

    let (count_header, count_style) = if !app.sort_by_size {
        (
            "Count ▼",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        ("Count  ", Style::default().fg(Color::Yellow))
    };

    // Bolt: Zero-allocation optimization: Use array instead of vec! to prevent heap allocations for table headers every render tick.
    let header_cells = [
        Cell::from("Backtrace (leaf <- root)").style(Style::default().fg(Color::Yellow)),
        Cell::from(
            ratatui::text::Line::from(size_header).alignment(ratatui::layout::Alignment::Right),
        )
        .style(size_style),
        Cell::from(
            ratatui::text::Line::from(count_header).alignment(ratatui::layout::Alignment::Right),
        )
        .style(count_style),
    ];

    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1)
        .bottom_margin(1);

    let rows: Vec<Row> = if items.is_empty() {
        let (msg, style) = if app.process_exited {
            (
                "✓ Zero leaks detected. No active allocations.".to_string(),
                Style::default().fg(Color::Green),
            )
        } else {
            let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let idx = (t / 100) as usize % spinner.len();
            (
                format!(
                    "{} No allocations tracked. Waiting for data...",
                    spinner[idx]
                ),
                Style::default().fg(Color::Gray),
            )
        };
        vec![Row::new([Cell::from(msg)]).style(style).height(1)]
    } else {
        items
            .iter()
            .map(|(trace, _size, _count, size_str, count_str)| {
                // Bolt: Zero-allocation optimization: Use array instead of vec! to prevent `O(N)` heap allocations per table row every render frame.
                let cells = [
                    // Zero-allocation: use as_str() instead of trace.clone() to prevent string allocation per table row every frame.
                    Cell::from(trace.as_str()),
                    Cell::from(
                        ratatui::text::Line::from(size_str.as_str())
                            .alignment(ratatui::layout::Alignment::Right),
                    ),
                    Cell::from(
                        ratatui::text::Line::from(count_str.as_str())
                            .alignment(ratatui::layout::Alignment::Right),
                    ),
                ];
                Row::new(cells).height(1)
            })
            .collect()
    };

    let sort_label = if app.sort_by_size { "Size" } else { "Count" };
    let base_title = if app.process_exited {
        "Unfreed Memory Leaks"
    } else {
        "Active Allocations"
    };
    let title_text = if let Some(selected) = app.table_state.selected() {
        format!(
            "{} ({} of {} items - Sorted by {})",
            base_title,
            (selected + 1).to_formatted_string(&Locale::en),
            items.len().to_formatted_string(&Locale::en),
            sort_label
        )
    } else {
        format!(
            "{} ({} items - Sorted by {})",
            base_title,
            items.len().to_formatted_string(&Locale::en),
            sort_label
        )
    };

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(70),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title_text)
            .title_bottom(Line::from(key_spans).alignment(ratatui::layout::Alignment::Right)),
    )
    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol(">> ")
    .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);

    f.render_stateful_widget(table, chunks[2], &mut app.table_state);

    if items.len() > chunks[2].height.saturating_sub(4) as usize {
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(items.len().saturating_sub(1))
            .position(app.table_state.selected().unwrap_or(0));

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        f.render_stateful_widget(
            scrollbar,
            chunks[2].inner(&ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn write_float_with_commas(w: &mut impl std::fmt::Write, val: f64) -> std::fmt::Result {
    let int_part = val.trunc() as u64;
    let frac_part = (val.fract() * 10.0).round() as u64;
    let mut buf = num_format::Buffer::default();
    if frac_part == 10 {
        buf.write_formatted(&(int_part + 1), &Locale::en);
        write!(w, "{}.0", buf.as_str())
    } else {
        buf.write_formatted(&int_part, &Locale::en);
        write!(w, "{}.{}", buf.as_str(), frac_part)
    }
}

pub(crate) fn write_bytes(w: &mut impl std::fmt::Write, v: f64) -> std::fmt::Result {
    if v < 1024.0 {
        let mut buf = num_format::Buffer::default();
        buf.write_formatted(&(v as u64), &Locale::en);
        write!(w, "{} B", buf.as_str())
    } else if v < 1024.0 * 1024.0 {
        write_float_with_commas(w, v / 1024.0)?;
        write!(w, " KB")
    } else if v < 1024.0 * 1024.0 * 1024.0 {
        write_float_with_commas(w, v / (1024.0 * 1024.0))?;
        write!(w, " MB")
    } else {
        write_float_with_commas(w, v / (1024.0 * 1024.0 * 1024.0))?;
        write!(w, " GB")
    }
}

fn format_bytes(v: f64) -> String {
    let mut s = String::new();
    let _ = write_bytes(&mut s, v);
    s
}
