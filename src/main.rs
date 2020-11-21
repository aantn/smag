mod ringbuffer;

use anyhow::Result;
use crossterm::event::{KeyEvent, KeyModifiers};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use histogram::Histogram;
use std::process::Command;
use std::io;
use std::io::Write;
use std::iter;
use std::ops::Add;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use structopt::StructOpt;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph};
use tui::{symbols, Terminal};

#[derive(Debug, StructOpt)]
#[structopt(name = "gping", about = "Ping, but with a graph.")]
struct Args {
    #[structopt(help = "Commands to run", required = true)]
    cmds: Vec<String>,
    #[structopt(
        short="t",
        long,
        default_value = "1.0",
        help = "Determines how frequently we run the command in seconds"
    )]
    polling_interval : f64,
    #[structopt(
        short="p",
        long,
        default_value = "100",
        help = "Determines the number of data points to display."
    )]
    buffer_size: usize,
}

struct App {
    styles: Vec<Style>,
    data: Vec<ringbuffer::FixedRingBuffer<(f64, f64)>>,
    buffer_size: usize,
    index: Vec<i64>,
    window_min: Vec<f64>,
    window_max: Vec<f64>,
}

impl App {
    fn new(host_count: usize, buffer_size: usize) -> Self {
        App {
            styles: (0..host_count)
                .map(|i| Style::default().fg(Color::Indexed(i as u8 + 1)))
                .collect(),
            data: (0..host_count)
                .map(|_| ringbuffer::FixedRingBuffer::new(buffer_size))
                .collect(),
            buffer_size,
            index: vec![0; host_count],
            window_min: vec![0.0; host_count],
            window_max: vec![buffer_size as f64; host_count],
        }
    }
    fn update(&mut self, cmd_index: usize, item: Option<f64>) {
        self.index[cmd_index] += 1;
        let data = &mut self.data[cmd_index];
        if data.len() >= self.buffer_size {
            self.window_min[cmd_index] += 1_f64;
            self.window_max[cmd_index] += 1_f64;
        }
        match item {
            Some(dur) => data.push((self.index[cmd_index] as f64, dur)),
            None => data.push((self.index[cmd_index] as f64, 0_f64)),
        }
    }
    fn stats(&self) -> Vec<Histogram> {
        self.data
            .iter()
            .map(|data| {
                let mut hist = Histogram::new();

                for (_, val) in data.iter().filter(|v| v.1 != 0f64) {
                    hist.increment(*val as u64).unwrap_or(());
                }

                hist
            })
            .collect()
    }
    fn x_axis_bounds(&self) -> [f64; 2] {
        [
            self.window_min.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
            self.window_max.iter().fold(0f64, |a, &b| a.max(b)),
        ]
    }
    fn y_axis_bounds(&self) -> [f64; 2] {
        let iter = self
            .data
            .iter()
            .map(|b| b.as_slice())
            .flatten()
            .map(|v| v.1);
        let min = iter.clone().fold(f64::INFINITY, |a, b| a.min(b));
        let max = iter.fold(0f64, |a, b| a.max(b));
        // Add a 10% buffer to the top and bottom
        let max_10_percent = (max * 10_f64) / 100_f64;
        let min_10_percent = (min * 10_f64) / 100_f64;
        [min - min_10_percent , max + max_10_percent ]
    }
    fn y_axis_labels(&self, bounds: [f64; 2]) -> Vec<Span> {
        // we want to generate 5 label ticks
        let min = bounds[0];
        let max = bounds[1];

        let difference = max - min;
        let increment = (difference / 3f64) as f64;
        let min = min as u64;

        (0..4)
            .map(|i| Span::raw(format!("{:?}", min.add((increment * i as f64) as u64))))
            .collect()
    }
}

#[derive(Debug)]
enum Event {
    Update(usize, Result<f64>),
    Input(KeyEvent),
}

fn run_command(cmd : &String) -> Result<f64> {
    let mut output = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C");
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-c");
        cmd
    };
    let output = output
        .arg(cmd)
        .output()?;

    let output = String::from_utf8_lossy(&output.stdout);
    let output : f64 = output.trim().parse()?;
    return Ok(output)
}

fn main() -> Result<()> {
    let args = Args::from_args();
    let mut app = App::new(args.cmds.len(), args.buffer_size);
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    let (key_tx, rx) = mpsc::channel();

    let mut threads = vec![];

    let quit_signal = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    for (cmd_id, cmd) in args.cmds.iter().cloned().enumerate() {
        let cmd_tx = key_tx.clone();

        let quit_signal_clone = std::sync::Arc::clone(&quit_signal);
        let polling_interval = Duration::from_millis((args.polling_interval * 1000 as f64) as u64);
        let cmd_thread = thread::spawn(move || -> Result<()> {
            while !quit_signal_clone.load(Ordering::Acquire) {
                let now = std::time::Instant::now();
                cmd_tx.send(Event::Update(cmd_id, run_command(&cmd)))?;
                let execution_time = now.elapsed();
                let time_to_sleep = polling_interval.checked_sub(execution_time);
                match time_to_sleep {
                    Some(duration) => thread::sleep(duration),
                    None => {}
                }
            }
            Ok(())
        });
        threads.push(cmd_thread);
    }

    let killed_thread = std::sync::Arc::clone(&quit_signal);
    let key_thread = thread::spawn(move || -> Result<()> {
        while !killed_thread.load(Ordering::Acquire) {
            if event::poll(Duration::from_secs(1))? {
                if let CEvent::Key(key) = event::read()? {
                    key_tx.send(Event::Input(key))?;
                }
            }
        }
        Ok(())
    });
    threads.push(key_thread);

    loop {
        match rx.recv()? {
            Event::Update(host_id, ping_result) => {
                match ping_result {
                    Ok(duration) => app.update(host_id, Some(duration)),
                    Err(_) => app.update(host_id, None),
                };
                terminal.draw(|f| {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(2)
                        .constraints(
                            iter::repeat(Constraint::Length(1))
                                .take(args.cmds.len())
                                .chain(iter::once(Constraint::Percentage(10)))
                                .collect::<Vec<_>>()
                                .as_ref(),
                        )
                        .split(f.size());
                    for (((cmd_id, cmd), stats), &style) in args
                        .cmds
                        .iter()
                        .enumerate()
                        .zip(app.stats())
                        .zip(&app.styles)
                    {
                        let header_layout = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints(
                                [
                                    Constraint::Percentage(25),
                                    Constraint::Percentage(25),
                                    Constraint::Percentage(25),
                                    Constraint::Percentage(25),
                                ]
                                    .as_ref(),
                            )
                            .split(chunks[cmd_id]);

                        f.render_widget(
                            Paragraph::new(format!("Running cmd: {}", cmd)).style(style),
                            header_layout[0],
                        );

                        f.render_widget(
                            Paragraph::new(format!(
                                "min {:?}",
                                stats.minimum().unwrap_or(0)
                            ))
                                .style(style),
                            header_layout[1],
                        );
                        f.render_widget(
                            Paragraph::new(format!(
                                "max {:?}",
                                stats.maximum().unwrap_or(0)
                            ))
                                .style(style),
                            header_layout[2],
                        );
                        f.render_widget(
                            Paragraph::new(format!(
                                "p95 {:?}",
                                stats.percentile(95.0).unwrap_or(0)
                            ))
                                .style(style),
                            header_layout[3],
                        );
                    }

                    let datasets: Vec<_> = app
                        .data
                        .iter()
                        .zip(&app.styles)
                        .map(|(data, &style)| {
                            Dataset::default()
                                .marker(symbols::Marker::Braille)
                                .style(style)
                                .graph_type(GraphType::Line)
                                .data(data.as_slice())
                        })
                        .collect();

                    let y_axis_bounds = app.y_axis_bounds();

                    let chart = Chart::new(datasets)
                        .block(Block::default().borders(Borders::NONE))
                        .x_axis(
                            Axis::default()
                                .style(Style::default().fg(Color::Gray))
                                .bounds(app.x_axis_bounds()),
                        )
                        .y_axis(
                            Axis::default()
                                .style(Style::default().fg(Color::Gray))
                                .bounds(y_axis_bounds)
                                .labels(app.y_axis_labels(y_axis_bounds)),
                        );
                    f.render_widget(chart, chunks[args.cmds.len()]);
                })?;
            }
            Event::Input(input) => match input.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    quit_signal.store(true, Ordering::Release);
                    break;
                }
                KeyCode::Char('c') if input.modifiers == KeyModifiers::CONTROL => {
                    quit_signal.store(true, Ordering::Release);
                    break;
                }
                _ => {}
            },
        }
    }

    for thread in threads {
        thread.join().unwrap()?;
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
