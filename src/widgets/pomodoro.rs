use crate::{
    common::Style,
    constants::TICK_VALUE_MS,
    events::{AppEventTx, TuiEvent, TuiEventHandler},
    storage::PomodoroRecord,
    utils::center,
    widgets::clock::{ClockState, ClockStateArgs, ClockWidget, Countdown},
};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{StatefulWidget, Widget},
};
use serde::{Deserialize, Serialize};
use std::{cmp::max, collections::HashMap, time::Duration};
use strum::Display;
use time::OffsetDateTime;

#[derive(Debug, Clone, Display, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub enum Mode {
    Work,
    Pause,
}

pub struct ClockMap {
    work: ClockState<Countdown>,
    pause: ClockState<Countdown>,
}

impl ClockMap {
    fn get_mut(&mut self, mode: &Mode) -> &mut ClockState<Countdown> {
        match mode {
            Mode::Work => &mut self.work,
            Mode::Pause => &mut self.pause,
        }
    }
    fn get(&self, mode: &Mode) -> &ClockState<Countdown> {
        match mode {
            Mode::Work => &self.work,
            Mode::Pause => &self.pause,
        }
    }
}

pub struct PomodoroState {
    mode: Mode,
    clock_map: ClockMap,
    round: u64,
    round_labels: HashMap<u64, String>,
    label_edit_mode: bool,
    label_before_edit: String,
    history: Vec<PomodoroRecord>,
}

pub struct PomodoroStateArgs {
    pub mode: Mode,
    pub initial_value_work: Duration,
    pub current_value_work: Duration,
    pub initial_value_pause: Duration,
    pub current_value_pause: Duration,
    pub with_decis: bool,
    pub app_tx: AppEventTx,
    pub round: u64,
    pub round_labels: HashMap<u64, String>,
    pub history: Vec<PomodoroRecord>,
}

impl PomodoroState {
    pub fn new(args: PomodoroStateArgs) -> Self {
        let PomodoroStateArgs {
            mode,
            initial_value_work,
            current_value_work,
            initial_value_pause,
            current_value_pause,
            with_decis,
            app_tx,
            round,
            round_labels,
            history,
        } = args;
        Self {
            mode,
            clock_map: ClockMap {
                work: ClockState::<Countdown>::new(ClockStateArgs {
                    initial_value: initial_value_work,
                    current_value: current_value_work,
                    tick_value: Duration::from_millis(TICK_VALUE_MS),
                    with_decis,
                    app_tx: Some(app_tx.clone()),
                })
                .with_name("Work".to_owned()),
                pause: ClockState::<Countdown>::new(ClockStateArgs {
                    initial_value: initial_value_pause,
                    current_value: current_value_pause,
                    tick_value: Duration::from_millis(TICK_VALUE_MS),
                    with_decis,
                    app_tx: Some(app_tx),
                })
                .with_name("Pause".to_owned()),
            },
            round,
            round_labels,
            label_edit_mode: false,
            label_before_edit: String::new(),
            history,
        }
    }

    fn get_clock_mut(&mut self) -> &mut ClockState<Countdown> {
        self.clock_map.get_mut(&self.mode)
    }

    pub fn get_clock(&self) -> &ClockState<Countdown> {
        self.clock_map.get(&self.mode)
    }

    pub fn get_clock_work(&self) -> &ClockState<Countdown> {
        &self.clock_map.work
    }

    pub fn get_clock_work_mut(&mut self) -> &mut ClockState<Countdown> {
        self.clock_map.get_mut(&Mode::Work)
    }

    pub fn get_clock_pause(&self) -> &ClockState<Countdown> {
        &self.clock_map.pause
    }

    pub fn get_clock_pause_mut(&mut self) -> &mut ClockState<Countdown> {
        self.clock_map.get_mut(&Mode::Pause)
    }

    pub fn get_mode(&self) -> &Mode {
        &self.mode
    }

    pub fn get_round(&self) -> u64 {
        self.round
    }

    pub fn get_label(&self) -> &str {
        self.round_labels
            .get(&self.round)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub fn get_round_labels(&self) -> &HashMap<u64, String> {
        &self.round_labels
    }

    pub fn get_history(&self) -> &Vec<PomodoroRecord> {
        &self.history
    }

    pub fn is_label_edit_mode(&self) -> bool {
        self.label_edit_mode
    }

    pub fn toggle_label_edit_mode(&mut self) {
        if !self.label_edit_mode {
            // Entering edit mode - save current label
            self.label_before_edit = self.get_label().to_string();
        }
        self.label_edit_mode = !self.label_edit_mode;
    }

    fn set_current_round_label(&mut self, label: String) {
        if label.is_empty() {
            self.round_labels.remove(&self.round);
        } else {
            self.round_labels.insert(self.round, label);
        }
    }

    pub fn set_with_decis(&mut self, with_decis: bool) {
        self.clock_map.work.with_decis = with_decis;
        self.clock_map.pause.with_decis = with_decis;
    }

    pub fn next(&mut self) {
        self.mode = match self.mode {
            Mode::Pause => Mode::Work,
            Mode::Work => Mode::Pause,
        };
    }

    fn record_completed_session(&mut self) {
        let record = PomodoroRecord {
            round: self.round,
            label: self.get_label().to_string(),
            mode: self.mode.clone(),
            duration: Duration::from(*self.get_clock().get_initial_value()),
            completed_at: OffsetDateTime::now_utc(),
        };
        self.history.push(record);
    }
}

impl TuiEventHandler for PomodoroState {
    fn update(&mut self, event: TuiEvent) -> Option<TuiEvent> {
        let edit_mode = self.get_clock().is_edit_mode();
        let label_edit_mode = self.is_label_edit_mode();
        match event {
            TuiEvent::Tick => {
                self.get_clock_mut().tick();
                self.get_clock_mut().update_done_count();
            }
            // LABEL EDIT mode
            TuiEvent::Key(key) if label_edit_mode => match key.code {
                KeyCode::Enter => {
                    // Save changes
                    self.toggle_label_edit_mode();
                }
                KeyCode::Esc => {
                    // Cancel changes - restore previous label
                    self.set_current_round_label(self.label_before_edit.clone());
                    self.toggle_label_edit_mode();
                }
                KeyCode::Char(c) => {
                    let mut label = self.get_label().to_string();
                    label.push(c);
                    self.set_current_round_label(label);
                }
                KeyCode::Backspace => {
                    let mut label = self.get_label().to_string();
                    label.pop();
                    self.set_current_round_label(label);
                }
                _ => return Some(event),
            },
            // CLOCK EDIT mode
            TuiEvent::Key(key) if edit_mode => match key.code {
                // Skip changes
                KeyCode::Esc => {
                    let clock = self.get_clock_mut();
                    // Important: set current value first
                    clock.set_current_value(*clock.get_prev_value());
                    // before toggling back to non-edit mode
                    clock.toggle_edit();
                }
                // Apply changes and update initial value
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.get_clock_mut().toggle_edit();
                    // update initial value
                    let c = *self.get_clock().get_current_value();
                    self.get_clock_mut().set_initial_value(c);
                }
                // Apply changes
                KeyCode::Char('s') => {
                    self.get_clock_mut().toggle_edit();
                }
                // Value up
                KeyCode::Up => {
                    self.get_clock_mut().edit_up();
                }
                // Value down
                KeyCode::Down => {
                    self.get_clock_mut().edit_down();
                }
                // move edit position to the left
                KeyCode::Left => {
                    self.get_clock_mut().edit_next();
                }
                // move edit position to the right
                KeyCode::Right => {
                    self.get_clock_mut().edit_prev();
                }
                _ => return Some(event),
            },
            // default mode
            TuiEvent::Key(key) => match key.code {
                // Toggle run/pause
                KeyCode::Char('s') => {
                    self.get_clock_mut().toggle_pause();
                }
                // Enter edit mode
                KeyCode::Char('e') => {
                    self.get_clock_mut().toggle_edit();
                }
                // Enter label edit mode
                KeyCode::Char('n') => {
                    self.toggle_label_edit_mode();
                }
                // toggle WORK/PAUSE
                KeyCode::Left => {
                    // `next` is acting as same as a "prev" function we don't have
                    self.next();
                }
                // toggle WORK/PAUSE
                KeyCode::Right => {
                    self.next();
                }
                // reset rounds AND clocks
                KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.round = 1;
                    self.get_clock_work_mut().reset();
                    self.get_clock_pause_mut().reset();
                }
                // reset current clock
                KeyCode::Char('r') => {
                    // Record completed work session before incrementing round
                    if self.get_mode() == &Mode::Work && self.get_clock().is_done() {
                        self.record_completed_session();
                        self.round += 1;
                    }
                    self.get_clock_mut().reset();
                }
                _ => return Some(event),
            },
            _ => return Some(event),
        }
        None
    }
}

pub struct PomodoroWidget {
    pub style: Style,
    pub blink: bool,
}

impl StatefulWidget for PomodoroWidget {
    type State = PomodoroState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let clock_widget = ClockWidget::new(self.style, self.blink);
        let label = Line::raw(
            (format!(
                "Pomodoro {} {}",
                state.mode.clone(),
                state.get_clock_mut().get_mode()
            ))
            .to_uppercase(),
        );
        // Display round label: show custom label if exists, otherwise "ROUND X"
        // When in edit mode, show cursor
        let label_round = if state.is_label_edit_mode() {
            Line::raw(format!("{}█", state.get_label()))
        } else if !state.get_label().is_empty() {
            Line::raw(state.get_label().to_string())
        } else {
            Line::raw((format!("round {}", state.get_round())).to_uppercase())
        };

        let area = center(
            area,
            Constraint::Length(max(
                clock_widget
                    .get_width(state.get_clock().get_format(), state.get_clock().with_decis),
                max(label.width() as u16, label_round.width() as u16),
            )),
            Constraint::Length(
                // empty line + height of `label` + `label_round`
                clock_widget.get_height() + 3,
            ),
        );

        let [v1, v2, v3, v4] = Layout::vertical(Constraint::from_lengths([
            1,
            clock_widget.get_height(),
            1,
            1,
        ]))
        .areas(area);

        // empty line keep everything in center vertically comparing to other
        // views (which have one label below the clock only)
        Line::raw("").centered().render(v1, buf);
        clock_widget.render(v2, buf, state.get_clock_mut());
        label.centered().render(v3, buf);
        label_round.centered().render(v4, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn default_pomodoro_args() -> PomodoroStateArgs {
        let (app_tx, _rx) = mpsc::unbounded_channel();
        PomodoroStateArgs {
            mode: Mode::Work,
            initial_value_work: Duration::from_secs(60 * 25),
            current_value_work: Duration::from_secs(60 * 25),
            initial_value_pause: Duration::from_secs(60 * 5),
            current_value_pause: Duration::from_secs(60 * 5),
            with_decis: false,
            app_tx,
            round: 1,
            round_labels: HashMap::new(),
            history: Vec::new(),
        }
    }

    #[test]
    fn test_label_starts_empty() {
        let state = PomodoroState::new(default_pomodoro_args());
        assert_eq!(state.get_label(), "");
    }

    #[test]
    fn test_label_initialization() {
        let (app_tx, _rx) = mpsc::unbounded_channel();
        let mut round_labels = HashMap::new();
        round_labels.insert(1, "Test task".to_string());
        let args = PomodoroStateArgs {
            round_labels,
            app_tx,
            ..default_pomodoro_args()
        };
        let state = PomodoroState::new(args);
        assert_eq!(state.get_label(), "Test task");
    }

    #[test]
    fn test_label_edit_mode_toggle() {
        let mut state = PomodoroState::new(default_pomodoro_args());
        assert!(!state.is_label_edit_mode());

        state.toggle_label_edit_mode();
        assert!(state.is_label_edit_mode());

        state.toggle_label_edit_mode();
        assert!(!state.is_label_edit_mode());
    }

    #[test]
    fn test_label_editing_via_events() {
        let mut state = PomodoroState::new(default_pomodoro_args());

        // Enter label edit mode
        state.toggle_label_edit_mode();
        assert!(state.is_label_edit_mode());

        // Type some characters
        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('t')
        )));
        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('e')
        )));
        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('s')
        )));
        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('t')
        )));

        assert_eq!(state.get_label(), "test");

        // Exit edit mode
        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Enter
        )));
        assert!(!state.is_label_edit_mode());
        assert_eq!(state.get_label(), "test");
    }

    #[test]
    fn test_label_backspace() {
        let mut state = PomodoroState::new(default_pomodoro_args());

        state.toggle_label_edit_mode();

        // Type "hello"
        for c in ['h', 'e', 'l', 'l', 'o'] {
            state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char(c)
            )));
        }
        assert_eq!(state.get_label(), "hello");

        // Backspace twice
        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Backspace
        )));
        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Backspace
        )));

        assert_eq!(state.get_label(), "hel");
    }

    #[test]
    fn test_label_escape_exits_edit_mode() {
        let mut state = PomodoroState::new(default_pomodoro_args());

        state.toggle_label_edit_mode();
        assert!(state.is_label_edit_mode());

        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Esc
        )));

        assert!(!state.is_label_edit_mode());
    }

    #[test]
    fn test_label_escape_cancels_changes() {
        let (app_tx, _rx) = mpsc::unbounded_channel();
        let mut round_labels = HashMap::new();
        round_labels.insert(1, "original".to_string());
        let args = PomodoroStateArgs {
            round_labels,
            app_tx,
            ..default_pomodoro_args()
        };
        let mut state = PomodoroState::new(args);
        assert_eq!(state.get_label(), "original");

        // Enter edit mode
        state.toggle_label_edit_mode();
        assert!(state.is_label_edit_mode());

        // Make some changes
        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('X')
        )));
        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('Y')
        )));
        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('Z')
        )));
        assert_eq!(state.get_label(), "originalXYZ");

        // Press Esc to cancel
        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Esc
        )));

        // Should exit edit mode and restore original label
        assert!(!state.is_label_edit_mode());
        assert_eq!(state.get_label(), "original");
    }

    #[test]
    fn test_labels_per_round() {
        let (app_tx, _rx) = mpsc::unbounded_channel();
        let mut round_labels = HashMap::new();
        round_labels.insert(1, "Round 1 task".to_string());
        round_labels.insert(2, "Round 2 task".to_string());

        let args = PomodoroStateArgs {
            round: 1,
            round_labels,
            app_tx,
            ..default_pomodoro_args()
        };
        let mut state = PomodoroState::new(args);

        // Check round 1 label
        assert_eq!(state.get_label(), "Round 1 task");

        // Move to round 2
        state.round = 2;
        assert_eq!(state.get_label(), "Round 2 task");

        // Move to round 3 (no label)
        state.round = 3;
        assert_eq!(state.get_label(), "");
    }

    #[test]
    fn test_label_persists_per_round() {
        let mut state = PomodoroState::new(default_pomodoro_args());

        // Set label for round 1
        state.toggle_label_edit_mode();
        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('A')
        )));
        state.toggle_label_edit_mode();
        assert_eq!(state.get_label(), "A");

        // Move to round 2
        state.round = 2;
        assert_eq!(state.get_label(), ""); // No label for round 2 yet

        // Set label for round 2
        state.toggle_label_edit_mode();
        state.update(TuiEvent::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('B')
        )));
        state.toggle_label_edit_mode();
        assert_eq!(state.get_label(), "B");

        // Go back to round 1
        state.round = 1;
        assert_eq!(state.get_label(), "A"); // Round 1 label should still be there
    }
}
