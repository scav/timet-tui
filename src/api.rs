use std::sync::mpsc::Sender;

use chrono::Datelike;
use color_eyre::eyre::eyre;
use serde::Deserialize;

use crate::config::Config;
use crate::model::Message;

#[derive(Deserialize, Debug, Clone)]
pub struct TimetEntries {
    pub entries: Option<Vec<TimetEntry>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimetEntry {
    pub day_of_year: u32,
    pub year: i32,
    pub month: u32,
    pub iso_week_year: u32,
    pub iso_week: u32,
    pub week: u32,
    pub hours: f64,
    pub project_name: String,
    pub project_id: String,
}

#[derive(Debug, Clone)]
pub struct Api {
    endpoint: String,
    api_key: String,
}

impl Api {
    pub fn new(config: &Config) -> Self {
        let mut api_key = String::with_capacity(config.api.key.len());
        api_key.clone_from(&config.api.key);
        let endpoint = format!("https://{}", config.api.endpoint);
        Self { endpoint, api_key }
    }

    /// Get all months for given year
    pub fn get_year(
        &self,
        sender: &Sender<Message>,
        now: chrono::DateTime<chrono::Utc>,
        year: u32,
    ) -> color_eyre::Result<Vec<TimetEntry>> {
        let mut months = vec![];
        let month = now.month();
        // TODO: this can potentially break...
        for i in 1..=month {
            months.append(&mut self.get_month(year, i)?);
            sender.send(Message::RefreshProgressing(i))?;
        }

        Ok(months)
    }
    /// Returns a vec of all existing entries for the given year and month.
    /// If the result is empty an empty list is returned ()
    pub fn get_month(&self, year: u32, month: u32) -> color_eyre::Result<Vec<TimetEntry>> {
        let url = format!(
            "{}/entries-bymonth?year={year}&month={month}",
            self.endpoint
        );

        match minreq::get(&url)
            .with_timeout(5)
            .with_header("X-API-KEY", &self.api_key)
            .send()?
            .json::<TimetEntries>()?
            .entries
        {
            Some(e) => Ok(e),
            None => Err(eyre!("No entries in response {}", url)),
        }
    }
}

#[cfg(test)]
mod tests {}
