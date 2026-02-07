use crate::{
    events::{TuiEvent, TuiEventHandler},
    storage::PomodoroRecord,
    utils::center,
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Row, StatefulWidget, Table, Widget},
};

/// State for `HistoryWidget`
pub struct HistoryState {
    history: Vec<PomodoroRecord>,
}

pub struct HistoryStateArgs {
    pub history: Vec<PomodoroRecord>,
}

impl HistoryState {
    pub fn new(args: HistoryStateArgs) -> Self {
        let HistoryStateArgs { history } = args;
        Self { history }
    }

    pub fn set_history(&mut self, history: Vec<PomodoroRecord>) {
        self.history = history;
    }

    pub fn get_history(&self) -> &Vec<PomodoroRecord> {
        &self.history
    }
}

impl TuiEventHandler for HistoryState {
    fn update(&mut self, event: TuiEvent) -> Option<TuiEvent> {
        // History view doesn't handle any special keys, pass everything through
        Some(event)
    }
}

#[derive(Debug)]
pub struct HistoryWidget;

impl StatefulWidget for HistoryWidget {
    type State = HistoryState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let title = Line::raw("POMODORO HISTORY");

        // Build table rows from history
        let header_row = Row::new(vec![
            Cell::from("Round"),
            Cell::from("Task"),
            Cell::from("Mode"),
            Cell::from("Duration"),
            Cell::from("Completed"),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = state
            .history
            .iter()
            .rev() // Show most recent first
            .map(|record| {
                let duration_mins = record.duration.as_secs() / 60;
                let label = if record.label.is_empty() {
                    format!("ROUND {}", record.round)
                } else {
                    record.label.clone()
                };

                Row::new(vec![
                    Cell::from(record.round.to_string()),
                    Cell::from(label),
                    Cell::from(format!("{}", record.mode)),
                    Cell::from(format!("{}m", duration_mins)),
                    Cell::from(format!(
                        "{:04}-{:02}-{:02} {:02}:{:02}",
                        record.completed_at.year(),
                        u8::from(record.completed_at.month()),
                        record.completed_at.day(),
                        record.completed_at.hour(),
                        record.completed_at.minute()
                    )),
                ])
            })
            .collect();

        // Calculate content dimensions
        let table_height = (rows.len() + 2) as u16; // +2 for header and border
        let table_width = 80; // Fixed width for the table

        let area = center(
            area,
            Constraint::Length(table_width),
            Constraint::Length(table_height.min(area.height)),
        );

        let [title_area, table_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);

        // Render title
        title.centered().render(title_area, buf);

        // Render table
        let widths = [
            Constraint::Length(6),  // Round
            Constraint::Length(30), // Task
            Constraint::Length(6),  // Mode
            Constraint::Length(10), // Duration
            Constraint::Length(18), // Completed
        ];

        let table = Table::new(rows, widths)
            .header(header_row)
            .block(Block::default().borders(Borders::ALL));

        Widget::render(table, table_area, buf);
    }
}
