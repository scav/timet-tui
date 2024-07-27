#![allow(unused)]
use ::chrono::Datelike;
use chrono::NaiveDate;
use color_eyre::{eyre::eyre, Result};
use core::panic;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::api::TimetEntry;
use crate::config::Config;

#[derive(Debug, Clone)]
pub struct Store {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl Store {
    pub fn new(config: &Config) -> Result<Self> {
        let manager = SqliteConnectionManager::file(format!("{}/timet.db", config.config_location));
        let pool = r2d2::Pool::new(manager).unwrap();

        let s = Store { pool };
        s.create_db()?;

        Ok(s)
    }

    fn create_db(&self) -> Result<()> {
        let conn = &self.pool.get()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS entry (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    hours REAL NOT NULL,
    project_name TEXT NOT NULL,
    project_id TEXT NOT NULL
)",
            (), // empty list of parameters.
        )?;

        Ok(())
    }

    /// Simply count the total number of entries in the database
    pub fn entry_count(&self) -> Result<i32> {
        let conn = &self.pool.get()?;
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM entry")?;
        let result = stmt.query_row::<i32, _, _>([], |r| r.get(0))?;

        Ok(result)
    }

    /// Truncate the entries table before a refresh
    pub fn entry_truncate(&self) -> Result<usize> {
        let conn = &self.pool.get()?;
        let mut stmt = conn.prepare("DELETE FROM entry")?;
        let result = stmt.execute([])?;
        Ok(result)
    }

    pub fn insert(&self, mut items: Vec<TimetEntry>) -> Result<()> {
        let conn = &self.pool.get().unwrap();
        let mut stmt = conn.prepare(
            r#"
                INSERT INTO entry ( 
                    date,                   
                    hours, 
                    project_name,
                    project_id
                )
                VALUES (?1, ?2, ?3, ?4)
                "#,
        )?;

        for item in items {
            let date = match chrono::NaiveDate::from_yo_opt(item.year, item.day_of_year) {
                Some(m) => m,
                None => {
                    return Err(eyre!(
                        "Could not create date from {}-{}",
                        item.year,
                        item.day_of_year
                    ))
                }
            };
            stmt.execute((&date, &item.hours, &item.project_name, &item.project_id))?;
        }

        Ok(())
    }

    pub fn get_days(&self) -> Result<Vec<DayEntry>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            r#"SELECT
            id,
            date as "date: NaiveDate",
            hours,
            project_name,
            project_id
            FROM entry"#,
        )?;
        let result = stmt
            .query_map([], |row| {
                Ok(DayEntry {
                    id: row.get(0)?,
                    date: row.get(1)?,
                    hours: row.get(2)?,
                    project_name: row.get(3)?,
                    project_id: row.get(4)?,
                })
            })?
            .map(|result| result.map_err(|err| color_eyre::Report::new(err)))
            .collect::<Result<Vec<DayEntry>>>();
        result
    }

    pub fn get_yearly_overview(&self) -> Result<Vec<Year>> {
        let conn = &self.pool.get()?;
        let mut stmt = conn.prepare("SELECT hours FROM entry WHERE date LIKE ?1")?;

        let now = chrono::Utc::now();

        let mut years = vec![];
        for i in 1..=now.month() {
            let target_date = match NaiveDate::from_ymd_opt(now.year(), i, 1) {
                Some(m) => m,
                None => return Err(eyre!("Could not create date from {}-{i}-1", now.year())),
            };
            let mut current_month_year = target_date.format("%Y-%m").to_string();
            current_month_year.push_str("-%");

            let result = stmt.query_map(
                [(&current_month_year)],
                |row| -> Result<f32, rusqlite::Error> { row.get(0) },
            )?;
            let hour: f32 = result.into_iter().fold(0.0, |acc, e| acc + e.unwrap());

            let month_name = target_date.format("%B").to_string();
            years.push(Year {
                hours: hour,
                month: i,
                month_name,
            });
        }

        Ok(years)
    }

    pub fn get_month_overview(&self, month: u32, year: i32) -> Result<Vec<Month>> {
        let target_date = match NaiveDate::from_ymd_opt(year, month, 1) {
            Some(m) => m,
            None => return Err(eyre!("Could not create date from {year}-{month}-1")),
        };

        let mut current_month_year = target_date.format("%Y-%m").to_string();
        current_month_year.push_str("-%");

        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT date, project_name, hours FROM entry WHERE date LIKE ?1 AND hours IS NOT 0 ORDER BY date ASC",
        )?;
        let result = stmt
            .query_map([(&current_month_year)], |row| {
                Ok(Month {
                    date: row.get(0)?,
                    project_name: row.get(1)?,
                    hours: row.get(2)?,
                })
            })?
            .map(|result| result.map_err(|err| color_eyre::Report::new(err)))
            .collect();

        result
    }
}

/// EntryContainer is a simple generic container. Perhaps a bad idea?
pub struct EntryContainer<T> {
    begin: NaiveDate,
    end: NaiveDate,
    content: Vec<T>,
}
/// DayEntry is the basic representation of a day
#[derive(Debug)]
pub struct DayEntry {
    pub id: i64,
    pub date: NaiveDate,
    pub hours: f64,
    pub project_name: String,
    pub project_id: String,
}

#[derive(Debug)]
pub struct Entry {
    pub date: NaiveDate,
    pub hours: f32,
    pub project_name: String,
}

#[derive(Debug)]
pub struct Year {
    pub hours: f32,
    pub month: u32,
    pub month_name: String,
}

#[derive(Debug)]
pub struct Overview {
    pub content: Vec<OverviewContent>,
    pub sum: f32,
}

#[derive(Debug)]
pub struct OverviewContent {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub hours: f32,
    pub week: i32,
}

#[derive(Debug, PartialEq, PartialOrd)]
pub struct Month {
    pub date: NaiveDate,
    pub project_name: String,
    pub hours: f32,
}

#[cfg(test)]
mod tests {

    use r2d2_sqlite::SqliteConnectionManager;
    use rusqlite::Connection;

    use crate::api::{TimetEntries, TimetEntry};
    use crate::store::Store;

    fn create_timet_entries() -> Vec<TimetEntry> {
        let input = std::fs::read("testdata/april.json").unwrap();
        let entries: TimetEntries = serde_json::from_slice(&input).unwrap();

        entries.entries.unwrap()
    }

    fn create_store() -> Store {
        let manager = SqliteConnectionManager::memory();
        let pool = r2d2::Pool::new(manager).unwrap();
        Store { pool }
    }

    #[test]
    fn test_entry_count() {
        let mut store = create_store();
        store.create_db().unwrap();
        let items = create_timet_entries();
        store.insert(items).unwrap();

        let sum = store.entry_count().unwrap();
        assert_eq!(sum > 0, true);
    }

    #[test]
    fn test_entry_truncate() {
        let mut store = create_store();
        store.create_db().unwrap();
        let items = create_timet_entries();
        store.insert(items).unwrap();

        let mut sum = store.entry_count().unwrap();
        assert_eq!(&sum > &0, true);
        store.entry_truncate().unwrap();
        sum = store.entry_count().unwrap();
        assert_eq!(sum, 0);
    }
    #[test]
    fn test_store_insert() {
        let mut store = create_store();
        store.create_db().unwrap();
        let items = create_timet_entries();
        let result = store.insert(items);
        assert_eq!(result.is_ok(), true)
    }

    #[test]
    fn test_store_get_yearly_overview() {
        let mut store = create_store();
        store.create_db().unwrap();
        let items = create_timet_entries();
        store.insert(items).unwrap();
        let result = store.get_yearly_overview();
        dbg!(&result);
        assert_eq!(result.is_ok(), true);
    }
    #[test]
    fn test_store_get_month() {
        let mut store = create_store();
        store.create_db().unwrap();
        let items = create_timet_entries();
        store.insert(items).unwrap();

        let result = store.get_month_overview(4, 2024);
        assert_eq!(result.is_ok(), true);
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_store_get_days() {
        let mut store = create_store();
        store.create_db().unwrap();
        let items = create_timet_entries();
        store.insert(items).unwrap();

        let result = store.get_days();
        assert_eq!(result.is_ok(), true);
        let days = result.unwrap();
        assert_eq!(days.is_empty(), false);
    }

    #[test]
    fn test_fully() {
        let month = create_timet_entries();
        let month2 = create_timet_entries();

        let mut store = create_store();
        store.create_db().unwrap();
        store.insert(month);
        store.insert(month2);

        let result = store.get_days();
    }
}
