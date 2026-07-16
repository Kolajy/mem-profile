use crate::allocator::REGISTRY;
use crate::backtrace::symbolicate_frames;
use crate::snapshot::dump_to_file;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Cell, Chart, Dataset, GraphType, Row, Table, TableState},
    Frame, Terminal,
};
use std::{
    collections::HashMap,
    fs, io,
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
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    // Create app state
    let app = Arc::new(Mutex::new(App::new(pid)));
    let app_clone = Arc::clone(&app);

    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = Arc::clone(&is_running);

    let monitor_thread = thread::spawn(move || {
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 };
        let start_time = Instant::now();

        while is_running_clone.load(Ordering::Relaxed) {
            let is_paused;
            {
                let app = app_clone.lock().unwrap();
                is_paused = app.is_paused;
            }

            if !is_paused {
                if let Some(rss_bytes) = get_rss_bytes(pid, page_size) {
                    let mut app = app_clone.lock().unwrap();
                    let elapsed = start_time.elapsed().as_secs_f64();
                    app.rss_history.push((elapsed, rss_bytes as f64));
                    // keep only last N points to avoid unbounded growth
                    if app.rss_history.len() > 1000 {
                        app.rss_history.remove(0);
                    }
                } else {
                    // Process died or can't read
                    let mut app = app_clone.lock().unwrap();
                    app.process_exited = true;
                }
            }

            thread::sleep(Duration::from_millis(500));
        }
    });

    let res = run_app(&mut terminal, app, is_running.clone());

    // Restore terminal
    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .unwrap();
    terminal.show_cursor().unwrap();

    is_running.store(false, Ordering::Relaxed);
    let _ = monitor_thread.join();

    if let Err(err) = res {
        println!("{:?}", err);
    }
}

fn get_rss_bytes(pid: u32, page_size: u64) -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let statm_path = format!("/proc/{}/statm", pid);
        if let Ok(content) = fs::read_to_string(&statm_path) {
            if let Some(resident_str) = content.split_whitespace().nth(1) {
                if let Ok(resident) = resident_str.parse::<u64>() {
                    return Some(resident * page_size);
                }
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
    pid: u32,
    rss_history: Vec<(f64, f64)>, // (time_s, bytes)
    is_paused: bool,
    process_exited: bool,
    table_state: TableState,
    sort_by_size: bool, // true: size, false: count
    last_snapshot_time: Option<Instant>,
    last_snapshot_name: Option<String>,
}

impl App {
    fn new(pid: u32) -> Self {
        Self {
            pid,
            rss_history: Vec::new(),
            is_paused: false,
            process_exited: false,
            table_state: TableState::default(),
            sort_by_size: true,
            last_snapshot_time: None,
            last_snapshot_name: None,
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
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: Arc<Mutex<App>>,
    _is_running: Arc<AtomicBool>,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        let items;
        {
            let mut app_lock = app.lock().unwrap();
            items = get_active_allocations(app_lock.sort_by_size);

            terminal.draw(|f| ui(f, &mut app_lock, &items))?;
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                let mut app_lock = app.lock().unwrap();
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(());
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Char('p') | KeyCode::Char(' ') => {
                        app_lock.is_paused = !app_lock.is_paused;
                    }
                    KeyCode::Char('s') => {
                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        let name = format!("tui_snapshot_{}.txt", timestamp);
                        dump_to_file(Path::new(&name));
                        app_lock.last_snapshot_time = Some(Instant::now());
                        app_lock.last_snapshot_name = Some(name);
                    }
                    KeyCode::Char('r') => {
                        app_lock.sort_by_size = !app_lock.sort_by_size;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app_lock.next(items.len());
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app_lock.previous(items.len());
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

// Returns a list of (backtrace_string, total_size, count)
fn get_active_allocations(sort_by_size: bool) -> Vec<(String, usize, usize)> {
    crate::allocator::IN_ALLOCATOR.with(|in_alloc| {
        let was_in = in_alloc.get();
        in_alloc.set(true);

        let mut raw_allocs: HashMap<Vec<*mut std::ffi::c_void>, (usize, usize)> = HashMap::new();

        for shard_mutex in REGISTRY.get_shards() {
            if let Ok(shard) = shard_mutex.lock() {
                for (_, meta) in shard.iter() {
                    // Avoid unconditional clone() of the backtrace Vec by checking if it exists first.
                    if let Some(entry) = raw_allocs.get_mut(&meta.backtrace) {
                        entry.0 += meta.size;
                        entry.1 += 1;
                    } else {
                        raw_allocs.insert(meta.backtrace.clone(), (meta.size, 1));
                    }
                }
            }
        }

        let mut folded = HashMap::new();
        for (frames, (total_size, count)) in raw_allocs {
            let symbols = symbolicate_frames(&frames);
            let mut stack = Vec::new();
            for sym in symbols.iter().rev() {
                let name = sym.name.as_deref().unwrap_or("<unknown>");
                if name.contains("mem_profile::") || name.contains("backtrace::") {
                    continue;
                }
                stack.push(name.to_string());
            }
            if stack.is_empty() {
                stack.push("<unknown>".to_string());
            }
            let stack_str = stack.join(" -> ");
            let entry = folded.entry(stack_str).or_insert((0usize, 0usize));
            entry.0 += total_size;
            entry.1 += count;
        }

        let mut result: Vec<_> = folded.into_iter().map(|(k, v)| (k, v.0, v.1)).collect();

        if sort_by_size {
            result.sort_by(|a, b| b.1.cmp(&a.1));
        } else {
            result.sort_by(|a, b| b.2.cmp(&a.2));
        }

        in_alloc.set(was_in);
        result
    })
}

fn ui(f: &mut Frame, app: &mut App, items: &[(String, usize, usize)]) {
    if items.is_empty() {
        app.table_state.select(None);
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
    let status_span = if app.process_exited {
        Span::styled(
            " [PROCESS EXITED] ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )
    } else if app.is_paused {
        Span::styled(
            " [PAUSED] ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [RUNNING] ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    };

    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let mut spans = vec![
        Span::raw(format!(" Mem-Profile TUI | PID: {} ", app.pid)),
        status_span,
        Span::raw(" | Keys: "),
        Span::styled("[p/Space]", key_style),
        Span::raw(if app.is_paused {
            " resume, "
        } else {
            " pause, "
        }),
        Span::styled("[s]", key_style),
        Span::raw("napshot, "),
        Span::styled("[r]", key_style),
        Span::raw("e-sort, "),
        Span::styled("[q]", key_style),
        Span::raw("uit, "),
        Span::styled("[up/down]", key_style),
        Span::raw(" scroll "),
    ];

    if let Some(time) = app.last_snapshot_time {
        if time.elapsed() < Duration::from_secs(3) {
            let msg = if let Some(ref name) = app.last_snapshot_name {
                format!(" | Snapshot saved to {}! ", name)
            } else {
                " | Snapshot Saved! ".to_string()
            };
            spans.push(Span::styled(
                msg,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));
        }
    }

    let title = Line::from(spans);

    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title(title);

    let current_rss = if let Some(last) = app.rss_history.last() {
        format_bytes(last.1)
    } else {
        "N/A".to_string()
    };

    let info = ratatui::widgets::Paragraph::new(format!("Current RSS: {}", current_rss))
        .block(title_block)
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(info, chunks[0]);

    // Graph
    if app.rss_history.is_empty() {
        let block = Block::default()
            .title("RSS Timeline (Last 60s)")
            .borders(Borders::ALL);
        let info = ratatui::widgets::Paragraph::new("Waiting for initial memory reading...")
            .block(block)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(info, chunks[1]);
    } else {
        // Zero-allocation: take a reference to the slice instead of unconditionally cloning the entire history Vec every frame.
        let data: &[(f64, f64)] = &app.rss_history;
        let max_time = data.last().map(|d| d.0).unwrap_or(10.0).max(10.0);
        let min_time = if max_time > 60.0 {
            max_time - 60.0
        } else {
            0.0
        };

        let max_bytes = data
            .iter()
            .map(|d| d.1)
            .fold(0.0, f64::max)
            .max(1024.0 * 1024.0);

        let datasets = vec![Dataset::default()
            .name("RSS")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(data)];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title("RSS Timeline (Last 60s)")
                    .borders(Borders::ALL),
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
    let size_header = if app.sort_by_size { "Size ▼" } else { "Size" };
    let count_header = if !app.sort_by_size {
        "Count ▼"
    } else {
        "Count"
    };
    let header_cells = vec!["Backtrace", size_header, count_header]
        .into_iter()
        .map(|h| Cell::from(h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1)
        .bottom_margin(1);

    let rows: Vec<Row> = if items.is_empty() {
        vec![Row::new(vec![Cell::from(
            "No active allocations tracked yet. Waiting for memory activity...",
        )
        .style(Style::default().fg(Color::DarkGray))])
        .height(1)]
    } else {
        items
            .iter()
            .map(|(trace, size, count)| {
                let cells = vec![
                    // Zero-allocation: use as_str() instead of trace.clone() to prevent string allocation per table row every frame.
                    Cell::from(trace.as_str()),
                    Cell::from(format_bytes(*size as f64)),
                    Cell::from(count.to_string()),
                ];
                Row::new(cells).height(1)
            })
            .collect()
    };

    let sort_label = if app.sort_by_size { "Size" } else { "Count" };
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
            .title(format!("Active Allocations (Sorted by {})", sort_label)),
    )
    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, chunks[2], &mut app.table_state);
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
