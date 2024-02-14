use crate::datastore::DataStore;
use crate::Args;
use std::iter;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Style};
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph};
use tui::{symbols, Terminal};

pub fn draw_ui<T: tui::backend::Backend>(
    args: &Args,
    data_store: &DataStore,
    terminal: &mut Terminal<T>,
) {
    terminal
        .draw(|f| {
            let chunks: Vec<tui::layout::Rect> = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    iter::repeat(Constraint::Length(1))
                        .take(args.cmds.len())
                        .chain(iter::once(Constraint::Percentage(10)))
                        .collect::<Vec<Constraint>>(),
                )
                .split(f.size());
            for (((cmd_id, cmd), stats), &style) in args
                .cmds
                .iter()
                .enumerate()
                .zip(data_store.stats())
                .zip(&data_store.styles)
            {
                let header_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Percentage(40),
                            Constraint::Percentage(15),
                            Constraint::Percentage(15),
                            Constraint::Percentage(15),
                            Constraint::Percentage(15),
                        ]
                        .as_ref(),
                    )
                    .split(chunks[cmd_id]);

                f.render_widget(
                    Paragraph::new(format!("Running cmd: {}", cmd)).style(style),
                    header_layout[0],
                );

                f.render_widget(
                    Paragraph::new(format!("current {:?}", data_store.last(cmd_id) as u64))
                        .style(style),
                    header_layout[1],
                );

                f.render_widget(
                    Paragraph::new(format!("min {:?}", stats.minimum().unwrap_or(0))).style(style),
                    header_layout[2],
                );
                f.render_widget(
                    Paragraph::new(format!("max {:?}", stats.maximum().unwrap_or(0))).style(style),
                    header_layout[3],
                );
                f.render_widget(
                    Paragraph::new(format!("p95 {:?}", stats.percentile(95.0).unwrap_or(0)))
                        .style(style),
                    header_layout[4],
                );
            }

            let datasets: Vec<_> = data_store
                .data
                .iter()
                .zip(&data_store.styles)
                .map(|(data, &style)| {
                    Dataset::default()
                        .marker(symbols::Marker::Braille)
                        .style(style)
                        .graph_type(GraphType::Line)
                        .data(data.as_slice())
                })
                .collect();

            let (y_axis_bounds, y_axis_num_ticks) = if args.manual_range.is_empty() {
                // Automatic range and tick placement algorithm
                data_store.y_axis_bounds()
            } else {
                // User supplied range and possiby tick increment
                let mut parts = args.manual_range.split(',');
                let num_args = parts.clone().count();
                if num_args != 2 && num_args != 3 {
                    panic!("Invalid range format. Please use 'min,max' or 'min,max,increment'");
                }

                let min : f64 = parts.next().unwrap().parse().unwrap();
                let max : f64 = parts.next().unwrap().parse().unwrap();
                if min >= max {
                    panic!("Invalid range format. Make sure min is less than max.");
                }
                let increment : f64 = if num_args == 3 {
                    parts.next().unwrap().parse().unwrap()
                } else {
                    (max - min) / 4.0   // Default to targeting 5 ticks (= 4+1)
                }.min(max - min);   // Make sure increment is not greater than range
                let tick_count : i32 = ((max - min) / increment).round() as i32 + 1;

                ([min, max], tick_count)
            };

            let chart = Chart::new(datasets)
                .block(Block::default().borders(Borders::NONE))
                .x_axis(
                    Axis::default()
                        .style(Style::default().fg(Color::Gray))
                        .bounds(data_store.x_axis_bounds()),
                )
                .y_axis(
                    Axis::default()
                        .style(Style::default().fg(Color::Gray))
                        .bounds(y_axis_bounds)
                        .labels(data_store.y_axis_labels(y_axis_bounds, y_axis_num_ticks)),
                );
            f.render_widget(chart, chunks[args.cmds.len()]);
        })
        .expect("error drawing ui");
}
