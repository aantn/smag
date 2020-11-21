use super::ringbuffer;
use histogram::Histogram;
use std::ops::Add;
use tui::style::{Color, Style};
use tui::text::Span;

pub struct DataStore {
    pub styles: Vec<Style>,
    pub data: Vec<ringbuffer::FixedRingBuffer<(f64, f64)>>,
    buffer_size: usize,
    window_min: Vec<f64>,
    window_max: Vec<f64>,
}

impl DataStore {
    pub fn new(host_count: usize, buffer_size: usize) -> Self {
        DataStore {
            styles: (0..host_count)
                .map(|i| Style::default().fg(Color::Indexed(i as u8 + 1)))
                .collect(),
            data: (0..host_count)
                .map(|_| ringbuffer::FixedRingBuffer::new(buffer_size))
                .collect(),
            buffer_size,
            window_min: vec![0.0; host_count],
            window_max: vec![buffer_size as f64; host_count],
        }
    }
    pub fn update(&mut self, cmd_index: usize, x_index: u64, item: Option<f64>) {
        let data = &mut self.data[cmd_index];
        if data.len() >= self.buffer_size {
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
        let (_, l) = data.last();
        *l
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
    pub fn y_axis_labels(&self, bounds: [f64; 2]) -> Vec<Span> {
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
