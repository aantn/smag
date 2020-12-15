mod datastore;
mod ringbuffer;
mod ui;
use crate::datastore::DataStore;
use anyhow::Result;
use crossterm::event::{KeyEvent, KeyModifiers};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::io::Write;
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use structopt::StructOpt;
use tui::backend::CrosstermBackend;
use tui::Terminal;

#[derive(Clone, Debug, StructOpt)]
#[structopt(
    name = "smag",
    about = "Show Me A Graph - Like the `watch` command but with a graph of previous values."
)]
pub struct Args {
    #[structopt(help = "Command(s) to run", required = true)]
    cmds: Vec<String>,

    #[structopt(
        short = "n",
        long = "interval",
        default_value = "1.0",
        help = "Specify update interval in seconds."
    )]
    polling_interval: f64,

    #[structopt(
        short = "y",
        long = "y-label",
        default_value = "",
        help = "Label/units for y-axis (e.g. 'MB', 'Seconds')"
    )]
    y_label: String,

    #[structopt(
        short = "d",
        long = "diff",
        help = "Graph the diff of subsequent command outputs"
    )]
    diff: bool,

    #[structopt(
        short = "h",
        long = "history",
        default_value = "100",
        help = "Specify number of points to 'remember' and graph at once for each commands"
    )]
    buffer_size: usize,
}

#[derive(Debug)]
enum Event {
    Update(u64, usize, Result<f64>),
    Input(KeyEvent),
}

fn run_command(cmd: &str) -> Result<f64> {
    let mut output = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C");
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-c");
        cmd
    };
    let output = output.arg(cmd).output()?;

    let output = String::from_utf8_lossy(&output.stdout);
    let output: f64 = output.trim().parse()?;
    Ok(output)
}

fn main() -> Result<()> {
    let args = Args::from_args();
    let mut app = DataStore::new(args.clone());
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let crossterm = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(crossterm)?;

    terminal.clear()?;

    let (key_tx, rx) = mpsc::channel();

    let mut threads = vec![];

    let quit_signal = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    for (cmd_id, cmd) in args.cmds.iter().cloned().enumerate() {
        let cmd_tx = key_tx.clone();

        let quit_signal_clone = std::sync::Arc::clone(&quit_signal);
        let polling_interval = Duration::from_millis((args.polling_interval * 1000_f64) as u64);
        let diff_mode = args.diff;
        let cmd_thread = thread::spawn(move || -> Result<()> {
            let mut previous: f64 = 0 as f64;
            let mut idx = 0;
            while !quit_signal_clone.load(Ordering::Acquire) {
                let now = std::time::Instant::now();
                let result = run_command(&cmd);
                if diff_mode {
                    if let Result::Ok(val) = result {
                        if idx != 0 {
                            cmd_tx.send(Event::Update(idx, cmd_id, Ok(val - previous)))?;
                        }
                        previous = val;
                    }
                } else {
                    cmd_tx.send(Event::Update(idx, cmd_id, result))?;
                }
                let execution_time = now.elapsed();
                let time_to_sleep = polling_interval.checked_sub(execution_time);
                if let Some(duration) = time_to_sleep {
                    thread::sleep(duration)
                }
                idx += 1;
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
            Event::Update(x_index, host_id, cmd_result) => {
                match cmd_result {
                    Ok(duration) => app.update(host_id, x_index, Some(duration)),
                    Err(_) => app.update(host_id, x_index, None),
                };
                crate::ui::draw_ui(&args, &app, &mut terminal)
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
