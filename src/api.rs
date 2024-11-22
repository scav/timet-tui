use std::sync::mpsc::Sender;

use chrono::{DateTime, Datelike};
use color_eyre::eyre::eyre;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hours<'a> {
    pub project: &'a str,
    pub date: chrono::NaiveDate,
    pub hours: f32,
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
        Self {
            endpoint: config.api.endpoint.clone(),
            api_key,
        }
    }

    /// Get all months for given year
    pub fn get_year(
        &self,
        sender: &Sender<Message>,
        now: DateTime<chrono::Utc>,
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
        let url = format!("{}entries-bymonth?year={year}&month={month}", self.endpoint);

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

    pub fn post_hours(&self, hours: &Hours) -> color_eyre::Result<()> {
        let response = minreq::post(format!("{}entries-upsert-one", self.endpoint))
            .with_timeout(5)
            .with_header("X-API-KEY", &self.api_key)
            .with_json(hours)?
            .send()?;

        if response.status_code != 200 {
            return Err(eyre!(
                "respone contains invalid or unexpected tatus code\n{:?}\n{:?}",
                response.status_code,
                response.as_str()
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::api::{Api, Hours};
    use crate::config::{self, Config};

    #[ignore = "integration tests not yet planned"]
    #[test]
    fn post_hours_test() {
        let cfg = Config {
            version: "0-test",
            commit: "sdfsfds",
            config_location: "".to_string(),
            default_project: None,
            api: config::Api {
                endpoint: std::env::var("TIMET_API_ENDPOINT").unwrap(),
                key: std::env::var("TIMET_API_KEY").unwrap(),
            },
        };

        let api = Api::new(&cfg);

        let res = api.post_hours(&Hours {
            project: &std::env::var("TIMET_PROJECT").unwrap(),
            date: chrono::NaiveDate::from_ymd_opt(2024, 11, 30).unwrap(),
            hours: 0.0,
        });

        assert!(res.is_ok())
    }
}
