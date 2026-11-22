//! Poll a race meeting and report odds moves and scratchings.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::NaiveDate;

use crate::api::{self, Client};
use crate::history;

#[derive(Debug, Clone)]
pub struct WatchOptions {
    pub date: Option<NaiveDate>,
    pub venue: Option<String>,
    pub race: Option<i64>,
    pub interval: u64,
    pub count: u32,
    pub webhook: Option<String>,
    pub telegram: Option<String>,
    pub record: bool,
    pub db: Option<PathBuf>,
}

struct RaceState {
    runners: BTreeSet<i64>,
    odds: BTreeMap<String, String>,
}

fn change_line(changes: &[String]) -> String {
    if changes.is_empty() {
        "no changes".to_string()
    } else {
        changes.join("\n")
    }
}

async fn notify(options: &WatchOptions, text: &str) -> Result<()> {
    if let Some(url) = &options.webhook {
        let client = reqwest::Client::new();
        client
            .post(url)
            .json(&serde_json::json!({ "text": text, "source": "hkjc" }))
            .send()
            .await
            .with_context(|| format!("posting to {url}"))?
            .error_for_status()
            .with_context(|| format!("webhook {url} rejected the notification"))?;
    }
    if let Some(target) = &options.telegram {
        let (token, chat) = target
            .rsplit_once(':')
            .ok_or_else(|| anyhow::anyhow!("--telegram must be BOT_TOKEN:CHAT_ID"))?;
        let url = format!("https://api.telegram.org/bot{token}/sendMessage");
        let client = reqwest::Client::new();
        client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat,
                "text": text.chars().take(4000).collect::<String>(),
                "disable_web_page_preview": true,
            }))
            .send()
            .await
            .context("sending the Telegram notification")?
            .error_for_status()
            .context("Telegram rejected the notification")?;
    }
    Ok(())
}

/// Run the polling loop. `count == 0` polls until interrupted.
pub async fn run(client: &Client, options: WatchOptions) -> Result<()> {
    let mut previous: BTreeMap<i64, RaceState> = BTreeMap::new();
    let mut poll = 0u32;
    loop {
        client.clear_cache();
        let data = client
            .meetings(options.date, options.venue.as_deref())
            .await?;
        let meeting = data
            .pick(options.date, options.venue.as_deref())
            .ok_or_else(|| anyhow::anyhow!("no race meeting is currently published"))?;
        let meeting_date = api::parse_date(&meeting.date)?;
        let venue_code = meeting.venue_code.clone();
        let races: Vec<i64> = meeting
            .races
            .iter()
            .map(|race| race.no)
            .filter(|no| options.race.is_none_or(|wanted| wanted == *no))
            .collect();
        if races.is_empty() {
            anyhow::bail!(
                "race {} is not on the card; the meeting has {} races",
                options.race.unwrap_or_default(),
                meeting.races.len()
            );
        }
        let mut message = format!(
            "HKJC {} {} — {} race(s)\n",
            meeting.date,
            venue_code,
            races.len()
        );
        let mut changes = Vec::new();
        for race_no in races {
            let card = meeting
                .race(race_no)
                .map(|race| race.card(meeting))
                .ok_or_else(|| anyhow::anyhow!("race {race_no} disappeared from the card"))?;
            let runners: BTreeSet<i64> =
                card.runners.iter().map(|runner| runner.horse_no).collect();
            let odds = client
                .odds(
                    meeting_date,
                    &venue_code,
                    race_no,
                    Some(vec!["WIN".to_string()]),
                )
                .await?;
            let view = odds.view(meeting_date, &venue_code, race_no);
            if options.record {
                history::record_odds(&view, options.db.as_deref())?;
            }
            let mut current = BTreeMap::new();
            for pool in &view.pools {
                for entry in &pool.odds {
                    if !entry.combination.is_empty() && !entry.odds.is_empty() {
                        current.insert(
                            format!("{}:{}", pool.pool, entry.combination),
                            entry.odds.clone(),
                        );
                    }
                }
            }
            if let Some(state) = previous.get(&race_no) {
                for runner in state.runners.difference(&runners) {
                    changes.push(format!("race {race_no}: #{runner:02} scratched"));
                }
                for runner in runners.difference(&state.runners) {
                    changes.push(format!("race {race_no}: #{runner:02} added"));
                }
                for (combination, odds) in &current {
                    if let Some(before) = state.odds.get(combination)
                        && before != odds
                    {
                        changes.push(format!("race {race_no}: {combination} {before} -> {odds}"));
                    }
                }
            }
            previous.insert(
                race_no,
                RaceState {
                    runners,
                    odds: current,
                },
            );
        }
        if poll == 0 {
            message.push_str("watching for odds moves and scratchings\n");
        }
        message.push_str(&change_line(&changes));
        println!("{message}");
        if poll == 0 || !changes.is_empty() {
            notify(&options, &message).await?;
        }
        poll += 1;
        if options.count > 0 && poll >= options.count {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(options.interval.max(5))).await;
    }
    Ok(())
}
