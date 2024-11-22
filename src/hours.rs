use crate::api::Api;
use crate::model::Message;
use crate::store::Store;
use crate::ui::{centered_rect, BORDER_COLOR, POPUP_STYLE, SELECTED_COLOR};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::palette::tailwind;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Paragraph};
use ratatui::Frame;

#[derive(Debug)]
pub struct HoursModel {
    api: Api,
    store: Store,
    project: String,
    input: String,
    character_index: usize,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum HoursMessage {
    Open(String),
    Return,
}

impl HoursModel {
    pub fn new(api: Api, store: Store) -> Self {
        HoursModel {
            api,
            store,
            project: "".to_string(),
            input: String::new(),
            character_index: 0,
        }
    }

    fn input_to_float(&self) -> Result<f32> {
        match self.input.is_empty() {
            true => Err(color_eyre::Report::msg("hours cannot be empty")),
            false => self
                .input
                .parse::<f32>()
                .map_err(|err| color_eyre::Report::new(err)),
        }
    }

    fn add_hours(&mut self, hours: f32) -> Result<()> {
        self.api.post_hours(&crate::api::Hours {
            project: &self.project,
            date: chrono::Utc::now().date_naive(),
            hours,
        })?;

        self.store
            .insert_hours(&self.project, &hours, &chrono::Utc::now().date_naive())
    }

    fn enter_char(&mut self, new_char: char) {
        if self.input.is_empty() && new_char == '.' {
            return;
        }
        if new_char == '.' && self.input.contains('.') {
            return;
        }
        if new_char.is_numeric() || new_char == '.' {
            let index = self.byte_index();
            self.input.insert(index, new_char);
            self.move_cursor_right();
        }
    }

    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);

            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }
}

pub fn handle_key(key: KeyEvent, model: &mut HoursModel) -> Result<Option<Message>> {
    match key.code {
        KeyCode::Enter => {
            if model.input.is_empty() {
                return Ok(None);
            }
            model.add_hours(model.input_to_float()?)?;
            model.input.clear();
            model.character_index = 0;
            Ok(Some(Message::RefreshCompleted))
        }
        KeyCode::Char(c) => {
            model.enter_char(c);
            Ok(None)
        }
        KeyCode::Backspace => {
            model.delete_char();
            Ok(None)
        }

        _ => Ok(None),
    }
}

pub fn update(model: &mut HoursModel, msg: HoursMessage) -> Result<Option<Message>> {
    match msg {
        HoursMessage::Open(project) => {
            model.project = project;
            model.input.clear();
            model.character_index = 0;
            Ok(Some(Message::View(crate::model::ActiveView::LogHours)))
        }
        _ => Ok(None),
    }
}

pub fn render(f: &mut Frame, model: &mut HoursModel, area: Rect) {
    let popup_area = centered_rect(40, 20, area);
    let popup = Block::bordered()
        .padding(Padding::new(2, 2, 2, 2))
        .title("Log todays hours")
        .style(POPUP_STYLE);

    f.render_widget(&popup, popup_area);
    let inner = popup.inner(popup_area);
    let vertical = Layout::vertical([Constraint::Length(3), Constraint::Length(5)]);
    let [input_area, help_area] = vertical.areas(inner);

    let input = Paragraph::new(model.input.as_str())
        .block(Block::bordered().title("Hours"))
        .style(BORDER_COLOR);

    f.set_cursor_position(Position::new(
        input_area.x + model.character_index as u16 + 1,
        input_area.y + 1,
    ));
    f.render_widget(input, input_area);

    let x = Span::styled(
        "Note!",
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(tailwind::RED.c200),
    );
    let text = vec![
        Line::from("Use only numbers and '.', e.g 7.5"),
        Line::from(""),
        Line::from(x).centered(),
        Line::from("Overrides daily hours for active project"),
        Line::from(""),
        Line::from("Add hours <Enter> ")
            .centered()
            .style(SELECTED_COLOR),
    ];

    let p = Paragraph::new(text);
    f.render_widget(p, help_area);
}
