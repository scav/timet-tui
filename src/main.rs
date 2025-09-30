use std::{
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

use color_eyre::Result;
use ratatui::crossterm::event::{self, Event, KeyCode};
use timet_tui::{
    api,
    config::Config,
    hours,
    model::{ActiveView, Message, Model, RunningState},
    project, store, tui,
    ui::view,
};

fn main() -> Result<()> {
    tui::install_panic_hook();
    let config = Config::new()?;
    let mut terminal = tui::init_terminal()?;
    let remote_api = api::Api::new(&config);
    let store = store::Store::new(&config)?;
    let (sender, receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
    let mut model = Model::new(sender.clone(), remote_api, store, config)?;

    while model.running_state != RunningState::Done {
        // Render the current view
        terminal.draw(|f| view(&mut model, f))?;

        // Handle events and map to a Message
        let mut current_msg = handle_event(&mut model, &receiver)?;

        // Process updates as long as they return a non-None message
        while current_msg.is_some() {
            current_msg = update(&mut model, current_msg.unwrap())?;
        }
    }

    tui::restore_terminal()?;
    Ok(())
}

/// Convert Event to Message
fn handle_event(model: &mut Model, receiver: &Receiver<Message>) -> Result<Option<Message>> {
    match receiver.try_recv() {
        Ok(m) => {
            return Ok(Some(m));
        }
        Err(_) => {
            // Simply go on
        }
    }

    if event::poll(Duration::from_millis(250))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                return handle_key(key, model);
            }
        }
    }
    Ok(None)
}

fn handle_key(key: event::KeyEvent, model: &mut Model) -> Result<Option<Message>> {
    match key.code {
        // Global keys
        KeyCode::Char('q') => Ok(Some(Message::Quit)),
        KeyCode::Char('H') => Ok(Some(Message::Home)),
        KeyCode::Char('l') => match model.active_project.clone() {
            Some(project) => Ok(Some(Message::AddHours(hours::HoursMessage::Open(
                project.project_id,
            )))),
            _ => {
                model.active_error_msg =
                    Some("An active project must be set to log hours".to_string());
                Ok(None)
            }
        },

        _ => {
            match model.active_view {
                ActiveView::LogHours => hours::handle_key(key, &mut model.add_hours_model),
                ActiveView::Hours => project::handle_key(key, &mut model.register_model),
                // this breaks detailMonth because it has no keys attached
                ActiveView::Home => match key.code {
                    KeyCode::Char('H') => Ok(Some(Message::Home)),
                    KeyCode::Char('p') => Ok(Some(Message::Hours(project::ProjectMessage::Open))),
                    KeyCode::Char('r') => Ok(Some(Message::RefreshStarted)),
                    KeyCode::Char('j') => {
                        model.next_row()?;
                        Ok(None)
                    }
                    KeyCode::Char('k') => {
                        model.previous_row()?;
                        Ok(None)
                    }
                    KeyCode::Enter => {
                        model.set_active_month()?;
                        Ok(Some(Message::DetailMonth))
                    }
                    _ => Ok(None),
                },
                _ => Ok(None),
            }
        }
    }
}

fn update(model: &mut Model, msg: Message) -> Result<Option<Message>> {
    match msg {
        Message::View(view) => {
            model.active_view = view;
            Ok(None)
        }
        Message::Home => {
            model.active_error_msg = None;
            Ok(Some(Message::View(ActiveView::Home)))
        }
        Message::DetailMonth => {
            model.set_active_month()?;
            Ok(Some(Message::View(ActiveView::Month)))
        }
        Message::RefreshStarted => {
            model.refresh();
            Ok(Some(Message::View(ActiveView::Loading)))
        }
        Message::RefreshProgressing(month) => {
            model.update_month = month;
            Ok(Some(Message::View(ActiveView::Loading)))
        }
        Message::RefreshCompleted => {
            model.overview = model.store.get_yearly_overview(model.active_year)?;
            Ok(Some(Message::View(ActiveView::Home)))
        }
        Message::RefreshFailed => {
            model.active_error_msg =
                Some(String::from("Could not refresh items - H(ome) or q(uit)"));
            Ok(None)
        }
        Message::Hours(m) => project::update(&mut model.register_model, m),
        Message::Quit => {
            model.running_state = RunningState::Done;
            Ok(None)
        }
        Message::ActiveProject(project) => match project {
            Some(p) => {
                model.store.insert_active_project(&p.project_id)?;
                model.active_project = Some(p);
                model.overview = model.store.get_yearly_overview(model.active_year)?;
                Ok(Some(Message::View(ActiveView::Home)))
            }
            None => {
                model.store.delete_active_project()?;
                model.active_project = None;
                model.overview = model.store.get_yearly_overview(model.active_year)?;
                Ok(Some(Message::View(ActiveView::Home)))
            }
        },
        Message::AddHours(hmsg) => hours::update(&mut model.add_hours_model, hmsg),
    }
}
