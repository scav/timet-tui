use std::{
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

use color_eyre::Result;
use ratatui::crossterm::event::{self, Event, KeyCode};
use timet_tui::{
    api,
    config::Config,
    model::{ActiveView, Message, Model, RunningState},
    store, tui,
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
        KeyCode::Char('H') => Ok(Some(Message::Home)),
        KeyCode::Char('r') => Ok(Some(Message::RefreshStarted)),
        KeyCode::Char('q') => Ok(Some(Message::Quit)),
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
    }
}

fn update(model: &mut Model, msg: Message) -> Result<Option<Message>> {
    match msg {
        Message::Home => model.active_view = ActiveView::Home,
        Message::DetailMonth => {
            model.set_active_month()?;
            model.active_view = ActiveView::Month;
        }
        Message::RefreshStarted => {
            model.active_view = ActiveView::Loading;
            model.refresh();
        }
        Message::RefreshProgressing(month) => {
            model.active_view = ActiveView::Loading;
            model.update_month = month;
        }
        Message::RefreshCompleted => {
            model.overview = model.store.get_yearly_overview()?;
            model.active_view = ActiveView::Home;
        }
        Message::Quit => {
            model.running_state = RunningState::Done;
        }
    };
    Ok(None)
}
