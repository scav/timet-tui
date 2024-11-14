use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

use chrono::Datelike;
use color_eyre::Result;
use ratatui::widgets::TableState;

use crate::api::Api;
use crate::config::Config;
use crate::store::{Month, Store, Year};

#[derive(Debug)]
pub struct Model {
    pub config: Config,
    pub sender: Sender<Message>,
    pub api: Arc<Api>,
    pub store: Arc<Store>,
    pub counter: i32,
    pub active_error_msg: Option<String>,
    pub running_state: RunningState,
    pub now: chrono::DateTime<chrono::Utc>,
    pub active_view: ActiveView,
    pub active_year: i32,
    pub active_month: u32,
    pub update_month: u32,
    pub overview: Vec<Year>,
    pub overview_month: Vec<Month>,
    pub table_state: TableState,
}

impl Model {
    pub fn new(sender: Sender<Message>, api: Api, store: Store, config: Config) -> Result<Self> {
        let now = chrono::Utc::now();
        let overview = store.get_yearly_overview()?;
        Ok(Model {
            config,
            sender,
            api: Arc::new(api),
            store: Arc::new(store),
            counter: 0,
            active_error_msg: None,
            running_state: RunningState::Running,
            now,
            active_view: ActiveView::Home,
            active_year: now.year(),
            active_month: 0,
            update_month: 0,
            overview,
            overview_month: vec![],
            table_state: TableState::default().with_selected(0),
        })
    }

    pub fn refresh(&self) {
        let api = self.api.clone();
        let store = self.store.clone();
        let sender = self.sender.clone();
        let now = self.now;
        thread::spawn(move || {
            match api.get_year(&sender, now, 2024) {
                Ok(items) => {
                    store.entry_truncate().unwrap();
                    store.insert(items).unwrap();
                    sender.send(Message::RefreshCompleted).unwrap();
                }
                Err(_) => sender.send(Message::RefreshFailed).unwrap(),
            };
        });
    }

    pub fn next_row(&mut self) -> Result<()> {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == self.overview.len() {
                    i
                } else {
                    i + 1
                }
            }

            None => 0,
        };
        self.set_active_month()?;
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
        self.set_active_month()
    }

    pub fn set_active_month(&mut self) -> Result<()> {
        self.active_month = self.table_state.selected().unwrap() as u32 + 1;
        self.overview_month = self.store.get_month_overview(self.active_month, 2024)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum ActiveView {
    Home,
    Loading,
    Month,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum RunningState {
    #[default]
    Running,
    Done,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Message {
    Home,
    RefreshStarted,
    RefreshProgressing(u32),
    RefreshCompleted,
    RefreshFailed,
    DetailMonth,
    Quit,
}
