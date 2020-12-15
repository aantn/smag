use super::ringbuffer;
use super::Args;
use histogram::Histogram;
use tui::style::{Color, Style};
use tui::text::Span;

pub struct DataStore {
    pub styles: Vec<Style>,
    pub data: Vec<ringbuffer::FixedRingBuffer<(f64, f64)>>,
    args : Args,
    window_min: Vec<f64>,
    window_max: Vec<f64>,
}

impl DataStore {
    pub fn new(args : Args) -> Self {
	let host_count = args.cmds.len();
        DataStore {
            styles: (0..host_count)
                .map(|i| Style::default().fg(Color::Indexed(i as u8 + 1)))
                .collect(),
            data: (0..host_count)
                .map(|_| ringbuffer::FixedRingBuffer::new(args.buffer_size))
                .collect(),
            window_min: vec![0.0; host_count],
            window_max: vec![args.buffer_size as f64; host_count],
            args: args,
        }
    }
    pub fn update(&mut self, cmd_index: usize, x_index: u64, item: Option<f64>) {
        let data = &mut self.data[cmd_index];
        if data.len() >= self.args.buffer_size {
            self.window_min[cmd_index] += 1_f64;
            self.window_max[cmd_index] += 1_f64;
        }
        match item {
            Some(val) => data.push((x_index as f64, val)),
            None => data.push((x_index as f64, 0_f64)),
        }
    }

    pub fn last(&self, cmd_index: usize) -> f64 {
        let data = &self.data[cmd_index];
        match data.len() {
            0 => 0 as f64,
            _ => {
                let (_, l) = data.last();
                *l
            }
        }
    }

    pub fn stats(&self) -> Vec<Histogram> {
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
    pub fn x_axis_bounds(&self) -> [f64; 2] {
        [
            self.window_min.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
            self.window_max.iter().fold(0f64, |a, &b| a.max(b)),
        ]
    }
    pub fn y_axis_bounds(&self) -> [f64; 2] {
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
        [min - min_10_percent, max + max_10_percent]
    }

    fn format_tick(&self, increment: f64, value: f64) -> String {
        if increment > 1.0 {
            format!("{:.0}", value)
        } else if increment < 1.0 && increment >= 0.1 {
            format!("{:.1}", value)
        } else if increment < 0.1 && increment >= 0.01 {
            format!("{:.2}", value)
        } else if increment < 0.01 && increment >= 0.001 {
            format!("{:.3}", value)
        } else if increment < 0.001 && increment >= 0.0001 {
            format!("{:.4}", value)
        } else if increment < 0.0001 && increment >= 0.00001 {
            format!("{:.5}", value)
        } else {
            format!("{}", value)
        }
    }

    pub fn y_axis_labels(&self, bounds: [f64; 2]) -> Vec<Span> {
	let ticks = 5;
        let min = bounds[0];
        let max = bounds[1];

        let difference = max - min;
        let increment = difference / (ticks as f64 - 1.0);

        (0..ticks)
            .map(|i| Span::raw(self.format_tick(increment, min + increment * i as f64) + " " + &self.args.y_label))
            .collect()
    }
}
