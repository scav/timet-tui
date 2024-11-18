use crate::model::{ActiveView, Message};
use crate::store::{Project, Store};
use crate::ui::{alternate_color, centered_rect, fill_color, SELECTED_COLOR};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Rect};
use ratatui::text::Text;
use ratatui::widgets::{Block, BorderType, Cell, Clear, HighlightSpacing, Row, Table, TableState};
use ratatui::Frame;

#[derive(Debug)]
pub struct ProjectModel {
    store: Store,
    pub table_state: TableState,
    pub projects: Vec<Project>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ProjectMessage {
    Open,
    Return,
}

impl ProjectModel {
    pub fn new(store: Store) -> Result<Self> {
        Ok(ProjectModel {
            store,
            table_state: TableState::default().with_selected(0),
            projects: Vec::with_capacity(4), // most users have at least 4 possible projects
        })
    }

    fn set_projects(&mut self) -> Result<()> {
        self.projects = self.store.projects()?;
        Ok(())
    }

    fn set_active_project(&self) -> Result<Project> {
        let project = self
            .projects
            .get(self.table_state.selected().unwrap())
            .unwrap();
        self.store.insert_active_project(&project.project_id)?;

        Ok(project.clone())
    }

    pub fn next_row(&mut self) -> Result<()> {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == self.projects.len() {
                    i
                } else {
                    i + 1
                }
            }

            None => 0,
        };
        self.table_state.select(Some(i));

        Ok(())
    }

    pub fn previous_row(&mut self) -> Result<()> {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i != 0 {
                    i - 1
                } else {
                    0
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        Ok(())
    }
}

pub fn handle_key(key: KeyEvent, model: &mut ProjectModel) -> Result<Option<Message>> {
    match key.code {
        KeyCode::Char('H') => Ok(Some(Message::Home)),
        KeyCode::Char('j') => {
            model.next_row()?;
            Ok(None)
        }
        KeyCode::Char('k') => {
            model.previous_row()?;
            Ok(None)
        }
        KeyCode::Char('x') => Ok(Some(Message::ActiveProject(None))),
        KeyCode::Enter => Ok(Some(Message::ActiveProject(Some(
            model.set_active_project()?,
        )))),
        _ => Ok(None),
    }
}

pub fn update(model: &mut ProjectModel, msg: ProjectMessage) -> Result<Option<Message>> {
    match msg {
        ProjectMessage::Return => Ok(Some(Message::View(ActiveView::Hours))),
        ProjectMessage::Open => {
            model.set_projects()?;
            Ok(Some(Message::View(ActiveView::Hours)))
        }
    }
}

pub fn render(f: &mut Frame, model: &mut ProjectModel, area: Rect) {
    f.render_widget(Clear, area);

    let area = centered_rect(40, 20, area);

    let header = ["Project"]
        .into_iter()
        .map(ratatui::widgets::Cell::from)
        .collect::<Row>()
        .style(alternate_color(1))
        .height(1);
    let rows = model.projects.iter().enumerate().map(|(i, data)| {
        vec![&data.project_name]
            .into_iter()
            .map(|content| Cell::from(Text::from(content.to_string())))
            .collect::<Row>()
            .style(alternate_color(i))
            .height(1)
    });
    let bar = " █ ";
    let t = Table::new(rows, [Constraint::Min(2), Constraint::Length(15 + 1)])
        .header(header)
        .block(
            Block::bordered()
                .border_type(BorderType::Plain)
                .title("Select active project"),
        )
        .row_highlight_style(SELECTED_COLOR)
        .highlight_symbol(Text::from(vec![bar.into(), bar.into()]))
        .footer(Row::new(vec![Cell::new("Set <Enter>   Unset <x>")]))
        .style(fill_color(model.projects.len()))
        .highlight_spacing(HighlightSpacing::Always);
    f.render_stateful_widget(t, area, &mut model.table_state);
}
