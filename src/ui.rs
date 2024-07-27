use crate::model::{ActiveView, Model};
use chrono::Datelike;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{
        palette::tailwind::{self},
        Style, Stylize,
    },
    text::Text,
    widgets::{
        Block, BorderType, Borders, Cell, Clear, HighlightSpacing, Padding, Paragraph, Row, Table,
    },
    Frame,
};

const BORDER_COLOR: Style = Style::new().fg(tailwind::INDIGO.c300);
const SELECTED_COLOR: Style = Style::new()
    .fg(tailwind::INDIGO.c950)
    .bg(tailwind::INDIGO.c300);

fn alternate_color(i: usize) -> Style {
    match i % 2 {
        0 => Style::default().bg(tailwind::INDIGO.c900),
        _ => Style::default().bg(tailwind::INDIGO.c950),
    }
}

// naive implementation of a filler based on colour
fn fill_color(i: usize) -> Style {
    match i % 2 {
        0 => Style::default().bg(tailwind::INDIGO.c950),
        _ => Style::default().bg(tailwind::INDIGO.c900),
    }
}

/// Renders the user interface widgets.
pub fn view(model: &mut Model, frame: &mut Frame) {
    let main_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ],
    )
    .split(frame.area());

    frame.render_widget(Clear, main_layout[0]);
    frame.render_widget(Clear, main_layout[1]);

    frame.render_widget(
        Block::new().borders(Borders::TOP).title("| timet "),
        main_layout[0],
    );
    frame.render_widget(
        Block::new().borders(Borders::TOP).title(format!(
            "| release: {}-{} | config: {} ",
            model.config.version, model.config.commit, model.config.config_location,
        )),
        main_layout[2],
    );

    let inner_layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(80), Constraint::Percentage(20)],
    )
    .split(main_layout[1]);

    render_help(frame, inner_layout[1]);

    let inner_overview = Layout::new(
        Direction::Vertical,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .split(inner_layout[0]);

    match model.active_view {
        ActiveView::Loading => {
            render_loading(frame, model);
        }
        ActiveView::Home => {
            render_home(frame, model, inner_overview[0]);
        }
        ActiveView::Month => {
            render_home(frame, model, inner_overview[0]);
            render_active_month(frame, model, inner_overview[1]);
        }
    }
}

fn render_help(f: &mut Frame, area: Rect) {
    let key_style = Style::default().blue();

    let header = ["Key", "Operation"]
        .into_iter()
        .map(ratatui::widgets::Cell::from)
        .collect::<Row>()
        .height(2);

    let key_map = vec![
        ("h", "Home screen"),
        ("o", "Overview"),
        ("q", "Quit application"),
        ("r", "Refresh database"),
        ("k", "Up"),
        ("j", "Down"),
        ("Enter", "Select"),
    ];

    let keys = key_map
        .iter()
        .map(|c| {
            Row::new(vec![
                Cell::from(Text::from(c.0)).style(key_style),
                Cell::from(Text::from(c.1)).style(key_style),
            ])
            .height(1)
        })
        .collect::<Vec<Row>>();

    let t = Table::new(keys, [Constraint::Min(2), Constraint::Length(15 + 1)])
        .header(header)
        .block(
            Block::bordered()
                .border_type(BorderType::Plain)
                .style(BORDER_COLOR)
                .title("Help"),
        )
        .highlight_spacing(HighlightSpacing::Always);

    f.render_widget(t, area);
}

fn render_loading(f: &mut Frame, model: &mut Model) {
    let area = f.area();
    let block = Block::bordered().padding(Padding::new(5, 10, 1, 2));
    let paragraph = Paragraph::new(format!(
        "Rebuilding database ({}/{} months)",
        model.update_month,
        model.now.month()
    ))
    .centered()
    .block(block);
    let area = centered_rect(60, 20, area);
    f.render_widget(Clear, area); //this clears out the background
    f.render_widget(paragraph, area);
}

/// Aligns the popup to the center of the view
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

fn render_home(f: &mut Frame, model: &mut Model, area: Rect) {
    let header = ["Month", "Hours"]
        .into_iter()
        .map(ratatui::widgets::Cell::from)
        .collect::<Row>()
        .style(alternate_color(1))
        .bottom_margin(1)
        .height(1);
    let rows = model.overview.iter().enumerate().map(|(i, data)| {
        vec![&data.month_name, &data.hours.to_string()]
            .into_iter()
            .map(|content| Cell::from(Text::from(content.to_string())))
            .collect::<Row>()
            .style(alternate_color(i))
            .height(2)
    });
    let bar = " █ ";
    let t = Table::new(rows, [Constraint::Min(2), Constraint::Length(15 + 1)])
        .header(header)
        .block(
            Block::bordered()
                .border_type(BorderType::Plain)
                .title(format!("{}", model.now.year())),
        )
        .row_highlight_style(SELECTED_COLOR)
        .highlight_symbol(Text::from(vec![bar.into(), bar.into()]))
        .style(fill_color(model.overview.len()))
        .highlight_spacing(HighlightSpacing::Always);

    f.render_stateful_widget(t, area, &mut model.table_state);
}

fn render_active_month(f: &mut Frame, model: &mut Model, area: Rect) {
    let month = chrono::NaiveDate::from_ymd_opt(model.active_year, model.active_month, 1)
        .unwrap()
        .format("%B");

    let header = ["Date", "Project", "Hours"]
        .into_iter()
        .map(ratatui::widgets::Cell::from)
        .collect::<Row>()
        .style(alternate_color(1))
        .bottom_margin(1)
        .height(1);

    let rows = model.overview_month.iter().enumerate().map(|(i, data)| {
        vec![
            data.date.format("%D").to_string(),
            data.project_name.to_string(),
            format!("{:.1}", data.hours),
        ]
        .into_iter()
        .map(|content| Cell::from(Text::from(content.to_string())))
        .collect::<Row>()
        .style(alternate_color(i))
        .height(1)
    });

    let t = Table::new(
        rows,
        [
            Constraint::Fill(1),
            Constraint::Fill(2),
            Constraint::Fill(3),
        ],
    )
    .header(header)
    .block(
        Block::bordered()
            .border_type(BorderType::Plain)
            .borders(Borders::TOP | Borders::BOTTOM)
            .title(format!("{}", month)),
    )
    // .style(fill_color(model.overview_month.len()))
    .style(fill_color(1))
    .highlight_spacing(HighlightSpacing::Always);

    f.render_widget(t, area);
}
