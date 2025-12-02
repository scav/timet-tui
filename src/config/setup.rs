use crossterm::event::{self, Event, KeyCode};
use eyre::Report;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Backend,
    style::{palette::tailwind, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph},
    Terminal,
};

use crate::ui::{BORDER_COLOR, POPUP_STYLE};

// This method spawns a separate UI in the terminal accepting the API key in order
// to avoid having API keys floating around within in terminal history.
pub fn enter_api_key(terminal: &mut Terminal<impl Backend>) -> Result<String, Report> {
    let mut input = String::new();

    // loop {
    //     terminal.draw(|f| {
    //         let area = f.size();
    //
    //         let masked: String = "*".repeat(input.len());
    //         let widget = Paragraph::new(masked);
    //
    //         f.render_widget(widget, area);
    //     })?;
    //
    //     if event::poll(std::time::Duration::from_millis(50))? {
    //         if let Event::Key(key) = event::read()? {
    //             match key.code {
    //                 KeyCode::Esc => break,
    //                 KeyCode::Enter => break,
    //                 KeyCode::Backspace => {
    //                     input.pop();
    //                 }
    //                 KeyCode::Char(c) => {
    //                     input.push(c);
    //                 }
    //                 _ => {}
    //             }
    //         }
    //     }
    // }

    // loop {
    //     terminal.draw(|f| {
    //         let area = f.area();
    //
    //         // Top-level layout: header, form, footer
    //         let chunks = Layout::default()
    //             .direction(Direction::Vertical)
    //             .constraints([
    //                 Constraint::Length(3), // header
    //                 Constraint::Min(3),    // form
    //                 Constraint::Length(2), // footer
    //             ])
    //             .split(area);
    //
    //         // ---------- Header ----------
    //         let header = Paragraph::new(" Secure Login Form ")
    //             .alignment(Alignment::Center)
    //             .style(
    //                 Style::default()
    //                     .fg(Color::Yellow)
    //                     .add_modifier(Modifier::BOLD),
    //             );
    //         f.render_widget(header, chunks[0]);
    //
    //         // ---------- Password Input ----------
    //         let masked = "*".repeat(input.len());
    //         let pw_widget = Paragraph::new(masked)
    //             .block(Block::default().title("Password").borders(Borders::ALL));
    //         f.render_widget(pw_widget, chunks[1]);
    //
    //         // ---------- Footer ----------
    //         let footer = Paragraph::new("Press Enter to submit, Esc to cancel")
    //             .alignment(Alignment::Center)
    //             .style(Style::default().fg(Color::DarkGray));
    //         f.render_widget(footer, chunks[2]);
    //     })?;
    //
    //     // Input handling
    //     if event::poll(std::time::Duration::from_millis(60))? {
    //         if let Event::Key(k) = event::read()? {
    //             match k.code {
    //                 KeyCode::Esc => break,
    //                 KeyCode::Enter => break,
    //                 KeyCode::Backspace => {
    //                     input.pop();
    //                 }
    //                 KeyCode::Char(c) => input.push(c),
    //                 _ => {}
    //             }
    //         }
    //     }
    // }

    loop {
        terminal.draw(|f| {
            let area = f.area();

            let popup_area = centered_rect(50, 30, area);

            let container = Block::default()
                .borders(Borders::ALL)
                .title("Timet.io Credentials")
                .border_style(BORDER_COLOR)
                .border_type(BorderType::Rounded);
            f.render_widget(&container, popup_area);

            let inner = container.inner(popup_area);
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // header
                    Constraint::Length(3), // input input
                    Constraint::Length(1), // spacer
                    Constraint::Length(5), // footer
                ])
                .margin(1)
                .split(inner);

            let masked = "*".repeat(input.len());
            let pw = Paragraph::new(masked).block(
                Block::default()
                    .border_style(BORDER_COLOR)
                    .borders(Borders::ALL)
                    .title("API Key"),
            );
            f.render_widget(pw, chunks[1]);

            let help_text = vec![
                Line::from("Where do I find my API Key?"),
                Line::from("Go to timet -> Settings -> API keys -> Generate"),
                Line::from("Paste key into this window and press"),
                Line::from("<Enter> ").centered().style(POPUP_STYLE),
            ];
            let footer = Paragraph::new(help_text)
                .alignment(Alignment::Center)
                .style(
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(tailwind::RED.c200),
                );

            f.render_widget(footer, chunks[3]);
        })?;

        if event::poll(std::time::Duration::from_millis(60))? {
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => break,
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(c) => input.push(c),
                    _ => {}
                }
            }
        }
    }

    Ok(input)
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
