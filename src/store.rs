#![allow(unused)]
use ::chrono::Datelike;
use chrono::NaiveDate;
use color_eyre::{eyre::eyre, Result};
use core::panic;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, OptionalExtension};
use std::str::FromStr;

use crate::api::TimetEntry;
use crate::config::Config;

#[derive(Debug, Clone)]
pub struct Store {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl Store {
    pub fn new(config: &Config) -> Result<Self> {
        let manager = SqliteConnectionManager::file(format!("{}/timet.db", config.config_location));
        let pool = r2d2::Pool::new(manager)?;

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
    project_id TEXT NOT NULL,
    UNIQUE(date, project_id) ON CONFLICT REPLACE
)",
            (), // empty list of parameters.
        )?;

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
            )
            "#,
            (),
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

    pub fn default_project(&self) -> Result<Option<Project>> {
        let conn = &self.pool.get()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT project_id, project_name FROM entry 
            WHERE project_id = (SELECT value FROM config WHERE key = 'active_project')
             "#,
        )?;

        stmt.query_row([], |row| {
            Ok(Project {
                project_id: row.get(0)?,
                project_name: row.get(1)?,
            })
        })
        .optional()
        .map_err(|err| color_eyre::Report::new(err))
    }

    pub fn delete_active_project(&self) -> Result<()> {
        let conn = &self.pool.get()?;
        let mut stmt = conn.prepare("DELETE FROM config WHERE key = 'active_project'")?;
        let result = stmt.execute([])?;
        Ok(())
    }

    pub fn insert_active_project(&self, project_id: &str) -> Result<()> {
        let conn = &self.pool.get()?;
        let mut stmt = conn.prepare(
            r#"
            INSERT INTO config 
            (key, value) VALUES('active_project', $1)
            ON CONFLICT(key) DO UPDATE SET value = ($1);
            "#,
        )?;

        let result = stmt.execute([&(project_id)])?;
        Ok(())
    }

    pub fn insert(&self, mut items: Vec<TimetEntry>) -> Result<()> {
        let conn = &self.pool.get()?;
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

    pub fn insert_hours(&self, project_id: &str, hours: &f32, date: &NaiveDate) -> Result<()> {
        let project_name = self.get_project_name(project_id)?;

        let conn = &self.pool.get()?;
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

        stmt.execute((date, hours, project_name, project_id))?;

        Ok(())
    }

    #[allow(clippy::let_and_return)]
    pub fn projects(&self) -> Result<Vec<Project>> {
        let conn = self.pool.get()?;
        let mut stmt = conn
            .prepare("select DISTINCT project_id, project_name FROM entry WHERE hours IS NOT 0 ORDER BY project_name")?;

        let result = stmt
            .query_map([], |row| {
                Ok(Project {
                    project_id: row.get(0)?,
                    project_name: row.get(1)?,
                })
            })?
            .map(|result| result.map_err(|err| color_eyre::Report::new(err)))
            .collect::<Result<Vec<Project>>>();
        result
    }

    pub fn get_project_name(&self, project_id: &str) -> Result<String> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT project_name FROM entry WHERE project_id = ?")?;

        let result: String = stmt.query_row([project_id], |row| row.get(0))?;

        Ok(result)
    }

    #[allow(clippy::let_and_return)]
    pub fn get_days(&self) -> Result<Vec<DayEntry>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            r#"SELECT
            id,
            date as "date: NaiveDate",
            hours,
            project_name,
            project_id
            FROM entry
            "#,
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
        let mut stmt = conn.prepare(
            r#"
        WITH RECURSIVE months(m) AS (
            SELECT 1
            UNION ALL
            SELECT m + 1 FROM months WHERE m < 12
        )
        SELECT 
            printf('%02d', m) AS month,
            COALESCE(SUM(e.hours), 0) AS total_hour
        FROM 
            months
        LEFT JOIN entry e ON strftime('%Y', e.date) = strftime('%Y', 'now')
            AND strftime('%m', e.date) = printf('%02d', m)
            AND (project_id = (SELECT value FROM config WHERE key = 'active_project')
                 OR NOT EXISTS (SELECT 1 FROM config WHERE key = 'active_project'))
        GROUP BY m
        ORDER BY m;
        "#,
        )?;
        let data = stmt
            .query_map([], |row| {
                let month: u32 = row.get::<_, String>(0)?.parse().unwrap();
                let total_hours: f32 = row.get(1)?;
                let month_date = NaiveDate::from_ymd_opt(2024, month, 1).unwrap();
                Ok(Year {
                    hours: total_hours,
                    month,
                    month_name: month_date.format("%B").to_string(),
                })
            })?
            .collect::<Result<Vec<Year>, _>>()?;

        Ok(data)
    }

    #[allow(clippy::let_and_return)]
    pub fn get_month_overview(&self, month: u32, year: i32) -> Result<Vec<Month>> {
        let target_date = match NaiveDate::from_ymd_opt(year, month, 1) {
            Some(m) => m,
            None => return Err(eyre!("Could not create date from {year}-{month}-1")),
        };

        let mut current_month_year = target_date.format("%Y-%m").to_string();
        current_month_year.push_str("-%");

        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT date, project_name, hours 
            FROM entry 
            WHERE date LIKE ?1 
            AND (project_id = (SELECT value FROM config WHERE key = 'active_project')
                 OR NOT EXISTS (SELECT 1 FROM config WHERE key = 'active_project'))
            AND hours IS NOT 0 
            ORDER BY date ASC
            "#,
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
    pub hours: f32,
    pub project_name: String,
    pub project_id: String,
}

#[derive(Debug)]
pub struct Entry {
    pub date: NaiveDate,
    pub hours: f32,
    pub project_name: String,
}
#[derive(Debug, Clone)]
pub struct Project {
    pub project_id: String,
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
    fn test_store_default_project() {
        let mut store = create_store();
        store.create_db().unwrap();
        let insert = store.insert_active_project("foobar");
        assert_eq!(insert.is_ok(), true);

        let result = store.default_project();
        assert!(result.is_ok());
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
    fn test_store_insert_hours() {
        let mut store = create_store();
        store.create_db().unwrap();
        let items = create_timet_entries();
        store.insert(items.clone()).unwrap();
        let project = items.get(0).unwrap();
        let result =
            store.insert_hours(&project.project_id, &7.5, &chrono::Utc::now().date_naive());
        assert_eq!(result.is_ok(), true, "{:?}", &result)
    }

    #[test]
    fn test_store_projects() {
        let mut store = create_store();
        store.create_db().unwrap();
        let items = create_timet_entries();
        store.insert(items).unwrap();

        let projects = store.projects();
        assert!(projects.is_ok());
        assert!(
            !projects.unwrap().is_empty(),
            "should always return test projects"
        );
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
