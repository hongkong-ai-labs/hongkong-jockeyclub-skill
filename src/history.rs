//! Local SQLite history of odds snapshots.
//!
//! The database lives under the XDG data directory, or at `$HKJC_DB` when set.
//! Racing data is public; only the odds the CLI fetched are stored.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, params};

use crate::model::{HistoryEntry, HistoryPoint, HistoryView, OddsView};

/// Default database location.
pub fn default_path() -> PathBuf {
    if let Ok(value) = std::env::var("HKJC_DB")
        && !value.trim().is_empty()
    {
        return PathBuf::from(value);
    }
    if let Ok(value) = std::env::var("XDG_DATA_HOME")
        && !value.trim().is_empty()
    {
        return PathBuf::from(value).join("hkjc").join("history.sqlite");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("hkjc")
        .join("history.sqlite")
}

fn connect(path: Option<&Path>) -> Result<Connection> {
    let path = path.map(Path::to_path_buf).unwrap_or_else(default_path);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let connection = Connection::open(&path)
        .with_context(|| format!("opening the history database at {}", path.display()))?;
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS odds_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            captured_at TEXT NOT NULL,
            meeting_date TEXT NOT NULL,
            venue TEXT NOT NULL,
            race_no INTEGER NOT NULL,
            pool TEXT NOT NULL,
            combination TEXT NOT NULL,
            odds TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS odds_snapshots_lookup
            ON odds_snapshots (meeting_date, venue, race_no, captured_at);",
    )?;
    Ok(connection)
}

/// Store every odds entry in a view as one snapshot. Returns the row count.
pub fn record_odds(view: &OddsView, path: Option<&Path>) -> Result<usize> {
    let mut connection = connect(path)?;
    let captured_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let transaction = connection.transaction()?;
    let mut count = 0;
    {
        let mut statement = transaction.prepare(
            "INSERT INTO odds_snapshots
                (captured_at, meeting_date, venue, race_no, pool, combination, odds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for pool in &view.pools {
            for entry in &pool.odds {
                statement.execute(params![
                    captured_at,
                    view.date,
                    view.venue_code,
                    view.race_no,
                    pool.pool,
                    entry.combination,
                    entry.odds,
                ])?;
                count += 1;
            }
        }
    }
    transaction.commit()?;
    Ok(count)
}

/// Read the recorded odds for one race, first and last value per combination.
pub fn load(
    meeting_date: &str,
    venue: &str,
    race_no: i64,
    pool: Option<&str>,
    limit: usize,
    path: Option<&Path>,
) -> Result<HistoryView> {
    let connection = connect(path)?;
    let mut statement = connection.prepare(
        "SELECT captured_at, pool, combination, odds
         FROM odds_snapshots
         WHERE meeting_date = ?1 AND venue = ?2 AND race_no = ?3
           AND (?4 IS NULL OR pool = ?4)
         ORDER BY captured_at, id",
    )?;
    let pool_filter = pool.map(|value| value.to_uppercase());
    let rows = statement.query_map(
        params![meeting_date, venue.to_uppercase(), race_no, pool_filter],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    )?;

    let mut entries: Vec<HistoryEntry> = Vec::new();
    let mut snapshots: Vec<String> = Vec::new();
    for row in rows {
        let (captured_at, pool, combination, odds) = row?;
        if !snapshots.contains(&captured_at) {
            snapshots.push(captured_at.clone());
        }
        match entries
            .iter_mut()
            .find(|entry| entry.pool == pool && entry.combination == combination)
        {
            Some(entry) => {
                entry.last = odds.clone();
                entry.points.push(HistoryPoint { captured_at, odds });
            }
            None => entries.push(HistoryEntry {
                pool,
                combination,
                first: odds.clone(),
                last: odds.clone(),
                movement: 0.0,
                points: vec![HistoryPoint { captured_at, odds }],
            }),
        }
    }
    for entry in &mut entries {
        let first = entry.first.parse::<f64>().unwrap_or(0.0);
        let last = entry.last.parse::<f64>().unwrap_or(0.0);
        entry.movement = ((last - first) * 100.0).round() / 100.0;
    }
    entries.sort_by(|left, right| {
        left.pool
            .cmp(&right.pool)
            .then_with(|| left.combination.cmp(&right.combination))
    });
    if limit > 0 {
        entries.truncate(limit);
    }
    Ok(HistoryView {
        meeting_date: meeting_date.to_string(),
        venue: venue.to_uppercase(),
        race_no,
        snapshots: snapshots.len(),
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{OddsEntry, OddsPoolView, OddsView};

    fn sample_view(odds: &str) -> OddsView {
        OddsView {
            date: "2026-11-22".to_string(),
            venue_code: "ST".to_string(),
            venue: "Sha Tin".to_string(),
            race_no: 1,
            pools: vec![OddsPoolView {
                pool: "WIN".to_string(),
                name: "Win".to_string(),
                status: String::new(),
                sell_status: String::new(),
                last_update: String::new(),
                legs: vec![1],
                odds: vec![OddsEntry {
                    combination: "1".to_string(),
                    odds: odds.to_string(),
                    hot_favourite: false,
                    bankers: Vec::new(),
                }],
            }],
        }
    }

    #[test]
    fn records_and_reads_odds_movement() {
        let path = std::env::temp_dir().join(format!(
            "hkjc-history-test-{}-{:?}.sqlite",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_file(&path);
        record_odds(&sample_view("4.2"), Some(&path)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        record_odds(&sample_view("3.1"), Some(&path)).unwrap();
        let view = load("2026-11-22", "ST", 1, Some("WIN"), 10, Some(&path)).unwrap();
        assert_eq!(view.snapshots, 2);
        assert_eq!(view.entries.len(), 1);
        assert_eq!(view.entries[0].first, "4.2");
        assert_eq!(view.entries[0].last, "3.1");
        assert_eq!(view.entries[0].movement, -1.1);
        let _ = std::fs::remove_file(&path);
    }
}
