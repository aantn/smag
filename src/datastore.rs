use super::ringbuffer;
use super::Args;
use histogram::Histogram;
use tui::style::{Color, Style};
use tui::text::Span;

pub struct DataStore {
    pub styles: Vec<Style>,
    pub data: Vec<ringbuffer::FixedRingBuffer<(f64, f64)>>,
    args: Args,
    window_min: Vec<f64>,
    window_max: Vec<f64>,
}

impl DataStore {
    pub fn new(args: Args) -> Self {
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
            args,
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
    pub fn y_axis_bounds(&self, chart_height: i32) -> ([f64; 2], i32) {
        let iter = self.data.iter().flat_map(|b| b.as_slice()).map(|v| v.1);
        let min = iter.clone().fold(f64::INFINITY, |a, b| a.min(b));
        let max = iter.fold(0f64, |a, b| a.max(b));
        let range = max - min;

        // Parameters for automatic range and tick placement algorithm
        let range_buffer_percent_per_side = ((chart_height - 20).max(0) as f64 / 10.0).min(5.0);
        let target_lines_per_tick = 7.0;

        // Calculate tick spacing by rounding the log10 of the preferred increment
        let range_buffered = if range != 0.0 {
            range * (1.0 + 2.0 * range_buffer_percent_per_side / 100.0)
        } else {
            2.0
        };
        let target_num_ticks: f64 = ((chart_height - 1) as f64 / target_lines_per_tick).max(2.0);
        let preferred_increment: f64 = range_buffered / target_num_ticks;
        let log10_times3: i32 = (preferred_increment.log10() * 3.0).round() as i32;
        let exponent: i32 = log10_times3 / 3;
        let mut increment: f64 = 10_f64.powf(exponent as f64);
        // Adjust increment to a power-of-ten multiple of 1, 2 or 5
        match log10_times3 % 3 {
            -2 => increment /= 5.0,
            -1 => increment /= 2.0,
            0 => (),
            1 => increment *= 2.0,
            2 => increment *= 5.0,
            _ => increment = 1.0,
        }

        // Add buffer and round out to multiples of increment
        let range_buffer_per_side = if range > 0.0 {
            range_buffer_percent_per_side / 100.0 * range
        } else {
            1.0
        };
        let min_round = ((min - range_buffer_per_side) / increment).floor() * increment;
        let max_round = ((max + range_buffer_per_side) / increment).ceil() * increment;
        // Calculate number of ticks
        let mut num_ticks: i32 = ((max_round - min_round) / increment).round() as i32 + 1;

        if ((chart_height - 1) as f64 / num_ticks as f64) < (0.3 * target_lines_per_tick) {
            // Ticks are too close together, keep only min and max
            num_ticks = 2;
        }

        ([min_round, max_round], num_ticks)
    }

    fn format_tick(&self, increment: f64, value: f64) -> String {
        if increment >= 1.0 {
            format!("{:.0}", value)
        } else {
            let precision: usize = increment.log10().abs().ceil() as usize;
            format!("{:.precision$}", value)
        }
    }

    pub fn y_axis_labels(&self, bounds: [f64; 2], num_ticks: i32) -> Vec<Span> {
        let min = bounds[0];
        let max = bounds[1];

        let increment = (max - min) / (num_ticks - 1) as f64;

        let y_label = &self.args.y_label;
        let mut suffix = String::new();
        suffix.push(' ');
        if !y_label.is_empty() {
            suffix.push_str(y_label);
            suffix.push(' ');
        }

        (0..num_ticks)
            .map(|i| Span::raw(self.format_tick(increment, min + increment * i as f64) + &suffix))
            .collect()
    }
}
