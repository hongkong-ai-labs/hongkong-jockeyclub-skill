//! HKJC data access: the bet.hkjc.com GraphQL API and racing.hkjc.com results.
//!
//! The GraphQL endpoint accepts only whitelisted operation documents, so every
//! request uses one of the verbatim queries in [`crate::queries`].

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use chrono::{Datelike, NaiveDate, Utc};
use scraper::{Html, Selector};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::model::*;
use crate::queries;

const GRAPHQL: &str = "https://info.cld.hkjc.com/graphql/base/";
const RESULTS_PAGE: &str =
    "https://racing.hkjc.com/racing/information/English/Racing/LocalResults.aspx";

/// Venue code to human name.
pub fn venue_name(code: &str) -> String {
    match code {
        "ST" => "Sha Tin".to_string(),
        "HV" => "Happy Valley".to_string(),
        "CH" => "Conghua".to_string(),
        other if other.starts_with('S') && other.len() > 1 => format!("Simulcast {other}"),
        "" => String::new(),
        other => other.to_string(),
    }
}

/// Current date in Hong Kong (UTC+8).
pub fn hong_kong_today() -> NaiveDate {
    let offset = chrono::FixedOffset::east_opt(8 * 3600).expect("valid offset");
    Utc::now().with_timezone(&offset).date_naive()
}

/// Parse `YYYY-MM-DD`.
pub fn parse_date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("date must be YYYY-MM-DD, got {value:?}"))
}

fn compact(date: NaiveDate) -> String {
    format!("{:04}{:02}{:02}", date.year(), date.month(), date.day())
}

fn date_only(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 10 {
        trimmed[..10].to_string()
    } else {
        trimmed.to_string()
    }
}

fn hhmm(value: &str) -> String {
    let trimmed = value.trim();
    match trimmed.find('T') {
        Some(index) if trimmed.len() >= index + 6 => trimmed[index + 1..index + 6].to_string(),
        _ => trimmed.to_string(),
    }
}

/// Cached HKJC client.
pub struct Client {
    http: reqwest::Client,
    cache: Mutex<HashMap<String, (Instant, String)>>,
}

impl Client {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "hongkong-jockeyclub-skill/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/hongkong-ai-labs/hongkong-jockeyclub-skill)"
            ))
            .timeout(Duration::from_secs(45))
            .build()?;
        Ok(Self {
            http,
            cache: Mutex::new(HashMap::new()),
        })
    }

    async fn cached_text(&self, url: &str, ttl: Duration) -> Result<String> {
        if let Some((at, body)) = self.cache.lock().expect("cache poisoned").get(url).cloned()
            && at.elapsed() < ttl
        {
            return Ok(body);
        }
        let response = self
            .http
            .get(url)
            .header("Accept", "text/html,application/xhtml+xml")
            .send()
            .await
            .with_context(|| format!("requesting {url}"))?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            bail!("{url} returned HTTP {status}");
        }
        self.cache
            .lock()
            .expect("cache poisoned")
            .insert(url.to_string(), (Instant::now(), body.clone()));
        Ok(body)
    }

    async fn graphql<T: DeserializeOwned>(
        &self,
        operation: &str,
        query: &str,
        variables: Value,
        ttl: Duration,
    ) -> Result<T> {
        let key = format!("gql:{operation}:{variables}");
        if let Some((at, body)) = self
            .cache
            .lock()
            .expect("cache poisoned")
            .get(&key)
            .cloned()
            && at.elapsed() < ttl
        {
            return decode::<T>(&body).context("decoding cached HKJC response");
        }
        let response = self
            .http
            .post(GRAPHQL)
            .header("Accept", "application/json")
            .header("Referer", "https://bet.hkjc.com/")
            .header("Origin", "https://bet.hkjc.com")
            .json(&json!({
                "operationName": operation,
                "query": query,
                "variables": variables,
            }))
            .send()
            .await
            .context("contacting the HKJC GraphQL API")?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            bail!("HKJC GraphQL returned HTTP {status}: {}", body.trim());
        }
        let data = decode::<T>(&body).context("decoding the HKJC GraphQL response")?;
        self.cache
            .lock()
            .expect("cache poisoned")
            .insert(key, (Instant::now(), body));
        Ok(data)
    }

    /// Race meetings and their race cards.
    pub async fn meetings(
        &self,
        date: Option<NaiveDate>,
        venue: Option<&str>,
    ) -> Result<MeetingsData> {
        let date = date.map(|date| date.to_string());
        let venue = venue.map(|value| value.to_uppercase());
        self.graphql(
            "raceMeetings",
            queries::RACE_MEETINGS,
            json!({ "date": date, "venueCode": venue }),
            Duration::from_secs(20),
        )
        .await
    }

    /// Live odds for one race of the current meeting.
    pub async fn odds(
        &self,
        date: NaiveDate,
        venue: &str,
        race_no: i64,
        types: Option<Vec<String>>,
    ) -> Result<OddsData> {
        let types = types.unwrap_or_else(|| {
            vec![
                "WIN".to_string(),
                "PLA".to_string(),
                "QIN".to_string(),
                "QPL".to_string(),
            ]
        });
        self.graphql(
            "sp4e_racing_pmPool",
            queries::RACE_ODDS,
            json!({
                "date": date.to_string(),
                "venueCode": venue.to_uppercase(),
                "raceNo": race_no,
                "oddsTypes": types,
            }),
            Duration::from_secs(15),
        )
        .await
    }

    /// Mark Six draws, newest first.
    pub async fn mark_six(
        &self,
        last_n: Option<i64>,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<Draw>> {
        let variables = match (from, to) {
            (Some(from), Some(to)) => json!({
                "startDate": compact(from),
                "endDate": compact(to),
            }),
            (Some(from), None) => json!({
                "startDate": compact(from),
                "endDate": compact(hong_kong_today()),
            }),
            (None, Some(to)) => json!({
                "startDate": compact(to - chrono::Days::new(90)),
                "endDate": compact(to),
            }),
            _ => json!({ "lastNDraw": last_n.unwrap_or(10) }),
        };
        let data: DrawsData = self
            .graphql(
                "marksixResult",
                queries::MARK_SIX_DRAWS,
                variables,
                Duration::from_secs(120),
            )
            .await?;
        Ok(data.lottery_draws)
    }

    /// Historical result for one race, scraped from racing.hkjc.com.
    pub async fn race_results(
        &self,
        date: NaiveDate,
        venue: &str,
        race_no: i64,
    ) -> Result<RaceResultView> {
        let url = format!(
            "{RESULTS_PAGE}?RaceDate={:02}/{:02}/{:04}&Racecourse={}&RaceNo={race_no}",
            date.day(),
            date.month(),
            date.year(),
            venue.to_uppercase()
        );
        let html = self.cached_text(&url, Duration::from_secs(3600)).await?;
        let parsed = parse_results_page(&html, race_no)?;
        if parsed.placings.is_empty() && parsed.race_id.is_empty() {
            bail!(
                "no result found for {} at {} race {race_no}; check the date is a race day and the venue is ST or HV",
                date,
                venue.to_uppercase()
            );
        }
        Ok(RaceResultView {
            date: date.to_string(),
            venue: venue_name(&venue.to_uppercase()),
            race_no,
            ..parsed
        })
    }

    /// Published race meetings between two dates, read from the fixture calendar.
    pub async fn fixtures(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<Fixture>> {
        let mut fixtures = Vec::new();
        let mut cursor =
            NaiveDate::from_ymd_opt(from.year(), from.month(), 1).expect("first of month is valid");
        while cursor <= to {
            let url = format!(
                "https://racing.hkjc.com/racing/information/English/Racing/Fixture.aspx?CalYear={}&CalMonth={:02}",
                cursor.year(),
                cursor.month()
            );
            let html = self.cached_text(&url, Duration::from_secs(3600)).await?;
            fixtures.extend(parse_fixture_month(&html, cursor.year(), cursor.month()));
            cursor = if cursor.month() == 12 {
                NaiveDate::from_ymd_opt(cursor.year() + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(cursor.year(), cursor.month() + 1, 1)
            }
            .expect("next month is valid");
        }
        fixtures.retain(|fixture| {
            let date = date_only(&fixture.date);
            date >= from.to_string() && date <= to.to_string()
        });
        fixtures.sort_by(|left, right| {
            left.date
                .cmp(&right.date)
                .then_with(|| left.venue_code.cmp(&right.venue_code))
        });
        fixtures
            .dedup_by(|left, right| left.date == right.date && left.venue_code == right.venue_code);
        Ok(fixtures)
    }

    /// Football matches currently offered, with HAD odds.
    pub async fn football(&self) -> Result<Vec<FootballMatch>> {
        let data: FootballData = self
            .graphql(
                "matchList",
                queries::FOOTBALL_MATCHES,
                json!({
                    "startIndex": null,
                    "endIndex": null,
                    "startDate": null,
                    "endDate": null,
                    "matchIds": null,
                    "tournIds": null,
                    "fbOddsTypes": ["HAD"],
                    "fbOddsTypesM": ["HAD"],
                    "inplayOnly": null,
                    "featuredMatchesOnly": false,
                    "frontEndIds": null,
                    "earlySettlementOnly": false,
                    "showAllMatch": false,
                }),
                Duration::from_secs(25),
            )
            .await?;
        Ok(data.matches)
    }

    /// All odds pools for one football match.
    pub async fn football_odds(
        &self,
        match_id: &str,
        types: Option<Vec<String>>,
    ) -> Result<Vec<FootballMatch>> {
        let types = types.unwrap_or_else(|| {
            vec![
                "HAD".to_string(),
                "HHA".to_string(),
                "CRS".to_string(),
                "TTG".to_string(),
            ]
        });
        let data: FootballData = self
            .graphql(
                "lastOdds",
                queries::FOOTBALL_ODDS,
                json!({ "matchId": match_id, "fbOddsTypes": types }),
                Duration::from_secs(20),
            )
            .await?;
        Ok(data.matches)
    }

    /// One football match with its full odds board.
    pub async fn football_match(
        &self,
        match_id: &str,
        types: Option<Vec<String>>,
    ) -> Result<FootballMatchView> {
        let list = self.football().await?;
        let detail = self.football_odds(match_id, types).await?;
        let mut view = list
            .iter()
            .find(|item| item.id == match_id)
            .map(FootballMatch::view)
            .unwrap_or_else(|| FootballMatchView {
                id: match_id.to_string(),
                date: String::new(),
                kick_off: String::new(),
                status: String::new(),
                tournament: String::new(),
                home: String::new(),
                away: String::new(),
                home_score: String::new(),
                away_score: String::new(),
                venue: String::new(),
                odds: Vec::new(),
            });
        if detail
            .iter()
            .any(|match_item| !match_item.fo_pools.is_empty())
        {
            view.odds = detail
                .iter()
                .flat_map(|match_item| pool_views(&match_item.fo_pools))
                .collect();
        }
        if view.home.is_empty() && view.odds.is_empty() {
            bail!("no football match found for id {match_id}");
        }
        Ok(view)
    }

    /// Official jockey, trainer or horse profile page.
    pub async fn profile(&self, kind: ProfileKind, id: &str) -> Result<ProfileView> {
        let source = match kind {
            ProfileKind::Jockey => format!(
                "https://racing.hkjc.com/en-us/local/information/jockeyprofile?jockeyid={id}&Season=Current"
            ),
            ProfileKind::Trainer => format!(
                "https://racing.hkjc.com/en-us/local/information/trainerprofile?trainerid={id}&Season=Current"
            ),
            ProfileKind::Horse => {
                format!("https://racing.hkjc.com/en-us/local/information/horse?horseid={id}")
            }
        };
        let html = self.cached_text(&source, Duration::from_secs(3600)).await?;
        let view = parse_profile(&html, kind, id, &source);
        if view.name.is_empty() {
            bail!("no {} profile found for id {id}", kind.as_str());
        }
        Ok(view)
    }

    /// Find jockeys, trainers and horses in the current meeting by name.
    pub async fn lookup(&self, query: &str) -> Result<Vec<LookupHit>> {
        let data = self.meetings(None, None).await?;
        let meeting = data
            .pick(None, None)
            .ok_or_else(|| anyhow::anyhow!("no race meeting is currently published"))?;
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            bail!("query must not be empty");
        }
        let meeting_label = format!(
            "{} {}",
            date_only(&meeting.date),
            venue_name(&meeting.venue_code)
        );
        let mut hits: Vec<LookupHit> = Vec::new();
        let mut seen: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        for race in &meeting.races {
            for runner in &race.runners {
                if runner.name_en.to_lowercase().contains(&needle) || runner.name_ch.contains(query)
                {
                    let id = runner
                        .horse
                        .as_ref()
                        .map(|horse| {
                            if horse.id.is_empty() {
                                horse.code.clone()
                            } else {
                                horse.id.clone()
                            }
                        })
                        .unwrap_or_default();
                    if seen.insert(("horse".to_string(), id.clone())) {
                        hits.push(LookupHit {
                            kind: "horse".to_string(),
                            name: runner.name_en.clone(),
                            name_zh: runner.name_ch.clone(),
                            id,
                            meeting: meeting_label.clone(),
                            race_no: race.no,
                            horse: runner.name_en.clone(),
                        });
                    }
                }
                if let Some(person) = &runner.jockey
                    && (person.name_en.to_lowercase().contains(&needle)
                        || person.name_ch.contains(query))
                    && seen.insert(("jockey".to_string(), person.code.clone()))
                {
                    hits.push(LookupHit {
                        kind: "jockey".to_string(),
                        name: person.name_en.clone(),
                        name_zh: person.name_ch.clone(),
                        id: person.code.clone(),
                        meeting: meeting_label.clone(),
                        race_no: race.no,
                        horse: runner.name_en.clone(),
                    });
                }
                if let Some(person) = &runner.trainer
                    && (person.name_en.to_lowercase().contains(&needle)
                        || person.name_ch.contains(query))
                    && seen.insert(("trainer".to_string(), person.code.clone()))
                {
                    hits.push(LookupHit {
                        kind: "trainer".to_string(),
                        name: person.name_en.clone(),
                        name_zh: person.name_ch.clone(),
                        id: person.code.clone(),
                        meeting: meeting_label.clone(),
                        race_no: race.no,
                        horse: runner.name_en.clone(),
                    });
                }
            }
        }
        hits.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(hits)
    }
}

/// The current Hong Kong racing season runs from 1 September to 31 July.
pub fn season_bounds(today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let (start_year, end_year) = if today.month() >= 9 {
        (today.year(), today.year() + 1)
    } else {
        (today.year() - 1, today.year())
    };
    (
        NaiveDate::from_ymd_opt(start_year, 9, 1).expect("season start is valid"),
        NaiveDate::from_ymd_opt(end_year, 7, 31).expect("season end is valid"),
    )
}

fn is_venue_code(value: &str) -> bool {
    matches!(value, "ST" | "HV" | "CH")
        || (value.len() == 2 && value.starts_with('S') && value.as_bytes()[1].is_ascii_digit())
}

/// Extract race dates and venues from one fixture calendar page.
fn parse_fixture_month(html: &str, year: i32, month: u32) -> Vec<Fixture> {
    let marker = "class=\"f_fl f_fs14\">";
    let alt_marker = "alt=\"";
    let mut out = Vec::new();
    let mut position = 0;
    while let Some(found) = html[position..].find(marker) {
        let day_start = position + found + marker.len();
        let day_digits: String = html[day_start..]
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect();
        let Ok(day) = day_digits.parse::<u32>() else {
            position = day_start;
            continue;
        };
        let after_day = day_start + day_digits.len();
        let next = html[after_day..]
            .find(marker)
            .map(|offset| after_day + offset)
            .unwrap_or(html.len());
        let chunk = &html[after_day..next];
        let mut search = 0;
        while let Some(alt_found) = chunk[search..].find(alt_marker) {
            let value_start = search + alt_found + alt_marker.len();
            let value_end = chunk[value_start..]
                .find('"')
                .map(|offset| value_start + offset)
                .unwrap_or(value_start);
            let value = &chunk[value_start..value_end];
            if is_venue_code(value)
                && let Some(date) = NaiveDate::from_ymd_opt(year, month, day)
            {
                out.push(Fixture {
                    date: date.to_string(),
                    venue_code: value.to_string(),
                });
            }
            search = value_end.saturating_add(1);
            if search >= chunk.len() {
                break;
            }
        }
        position = next;
    }
    out
}

/// Filter the season fixture list and add weekday and venue names.
pub fn fixture_views(
    fixtures: &[Fixture],
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    venue: Option<&str>,
) -> Vec<FixtureView> {
    let venue = venue.map(|value| value.to_uppercase());
    let mut seen = std::collections::HashSet::new();
    let mut views: Vec<FixtureView> = fixtures
        .iter()
        .filter(|fixture| {
            let date = date_only(&fixture.date);
            from.is_none_or(|from| date >= from.to_string())
                && to.is_none_or(|to| date <= to.to_string())
                && venue
                    .as_deref()
                    .is_none_or(|venue| fixture.venue_code == venue)
        })
        .filter(|fixture| seen.insert((fixture.date.clone(), fixture.venue_code.clone())))
        .map(|fixture| {
            let date = date_only(&fixture.date);
            let day_of_week = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                .map(|parsed| parsed.format("%a").to_string())
                .unwrap_or_default();
            FixtureView {
                date,
                day_of_week,
                venue_code: fixture.venue_code.clone(),
                venue: venue_name(&fixture.venue_code),
            }
        })
        .collect();
    views.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| left.venue_code.cmp(&right.venue_code))
    });
    views
}

/// Human label for an HKJC football status code.
fn football_status(status: &str) -> String {
    match status {
        "PREEVENT" => "Not started".to_string(),
        "FIRSTHALF" => "1st half".to_string(),
        "FIRSTHALFCOMPLETED" => "Half time".to_string(),
        "SECONDHALF" => "2nd half".to_string(),
        "FULLTIME" | "ENDED" => "Full time".to_string(),
        "ABANDONED" => "Abandoned".to_string(),
        "POSTPONED" => "Postponed".to_string(),
        "" => "Unknown".to_string(),
        other => other.to_lowercase().replace('_', " "),
    }
}

fn pool_views(pools: &[FoPool]) -> Vec<FootballPoolView> {
    pools
        .iter()
        .filter(|pool| !pool.lines.is_empty() || !pool.closed_lines.is_empty())
        .map(|pool| FootballPoolView {
            pool: pool.odds_type.clone(),
            name: if pool.name_en.is_empty() {
                pool.odds_type.clone()
            } else {
                pool.name_en.clone()
            },
            status: pool.status.clone(),
            lines: pool
                .lines
                .iter()
                .map(|line| FootballLineView {
                    condition: text(&line.condition),
                    odds: line
                        .combinations
                        .iter()
                        .map(|combination| FootballOddView {
                            combination: combination.label.clone(),
                            label: if combination.selections.is_empty() {
                                combination.label.clone()
                            } else {
                                combination
                                    .selections
                                    .iter()
                                    .map(|selection| selection.label.clone())
                                    .collect::<Vec<_>>()
                                    .join("/")
                            },
                            odds: text(&combination.current_odds),
                            status: combination.status.clone(),
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

impl FootballMatch {
    pub fn view(&self) -> FootballMatchView {
        let score = self.running_result.as_ref();
        FootballMatchView {
            id: self.id.clone(),
            date: date_only(&self.match_date),
            kick_off: hhmm(&self.kick_off_time),
            status: football_status(&self.status),
            tournament: self
                .tournament
                .as_ref()
                .map(|tournament| tournament.name_en.clone())
                .unwrap_or_default(),
            home: self
                .home_team
                .as_ref()
                .map(|team| team.name_en.clone())
                .unwrap_or_default(),
            away: self
                .away_team
                .as_ref()
                .map(|team| team.name_en.clone())
                .unwrap_or_default(),
            home_score: score
                .map(|score| text(&score.home_score))
                .unwrap_or_default(),
            away_score: score
                .map(|score| text(&score.away_score))
                .unwrap_or_default(),
            venue: self
                .venue
                .as_ref()
                .map(|venue| {
                    if venue.name_en.is_empty() {
                        venue.code.clone()
                    } else {
                        venue.name_en.clone()
                    }
                })
                .unwrap_or_default(),
            odds: pool_views(&self.fo_pools),
        }
    }
}

/// Filter, sort and trim a football match list.
pub fn football_views(
    matches: &[FootballMatch],
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    tournament: Option<&str>,
    limit: usize,
) -> Vec<FootballMatchView> {
    let tournament = tournament.map(|value| value.to_lowercase());
    let mut views: Vec<FootballMatchView> = matches
        .iter()
        .filter(|item| {
            let date = date_only(&item.match_date);
            from.is_none_or(|from| date >= from.to_string())
                && to.is_none_or(|to| date <= to.to_string())
                && tournament.as_ref().is_none_or(|needle| {
                    item.tournament.as_ref().is_some_and(|tournament| {
                        tournament.name_en.to_lowercase().contains(needle)
                            || tournament.name_ch.contains(needle)
                            || tournament.code.to_lowercase().contains(needle)
                    })
                })
        })
        .map(FootballMatch::view)
        .collect();
    views.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| left.kick_off.cmp(&right.kick_off))
    });
    if limit > 0 {
        views.truncate(limit);
    }
    views
}

/// HKJC sends `null` for absent strings, lists and numbers. Dropping those keys
/// lets serde fall back to each field's default instead of failing the decode.
fn strip_nulls(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(_, value)| !value.is_null())
                .map(|(key, value)| (key, strip_nulls(value)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(strip_nulls).collect()),
        other => other,
    }
}

fn decode<T: DeserializeOwned>(body: &str) -> Result<T> {
    let value: Value = serde_json::from_str(body)?;
    let envelope: Envelope<T> = serde_path_to_error::deserialize(strip_nulls(value))
        .map_err(|error| anyhow::anyhow!("{} (at {})", error.inner(), error.path()))?;
    unwrap_envelope(envelope)
}

fn unwrap_envelope<T>(envelope: Envelope<T>) -> Result<T> {
    if let Some(data) = envelope.data {
        return Ok(data);
    }
    let message = envelope
        .errors
        .into_iter()
        .map(|error| error.message)
        .collect::<Vec<_>>()
        .join("; ");
    if message.is_empty() {
        bail!("HKJC returned an empty response");
    }
    bail!("HKJC API error: {message}")
}

// ---------------------------------------------------------------------------
// Meeting selection and views
// ---------------------------------------------------------------------------

impl MeetingsData {
    /// Pick the meeting matching the requested date/venue, else the first one.
    pub fn pick(&self, date: Option<NaiveDate>, venue: Option<&str>) -> Option<&Meeting> {
        let date = date.map(|date| date.to_string());
        let venue = venue.map(|value| value.to_uppercase());
        if let Some(found) = self.race_meetings.iter().find(|meeting| {
            date.as_deref().is_none_or(|date| meeting.date == date)
                && venue.as_deref().is_none_or(|v| meeting.venue_code == v)
        }) {
            return Some(found);
        }
        // A requested date or venue that is not in the live feed is a miss, not
        // a reason to answer with a different meeting.
        if date.is_none() && venue.is_none() {
            self.race_meetings.first()
        } else {
            None
        }
    }

    /// Meeting summaries, filtered by date/venue when requested.
    pub fn views_for(&self, date: Option<NaiveDate>, venue: Option<&str>) -> Vec<MeetingView> {
        let all = self.views();
        if date.is_none() && venue.is_none() {
            return all;
        }
        let date = date.map(|date| date.to_string());
        let venue = venue.map(|value| value.to_uppercase());
        all.into_iter()
            .filter(|meeting| {
                date.as_deref().is_none_or(|date| meeting.date == date)
                    && venue
                        .as_deref()
                        .is_none_or(|venue| meeting.venue_code == venue)
            })
            .collect()
    }

    /// Meeting summaries, falling back to the sparse active-meeting list.
    pub fn views(&self) -> Vec<MeetingView> {
        if !self.race_meetings.is_empty() {
            return self.race_meetings.iter().map(Meeting::view).collect();
        }
        self.active_meetings
            .iter()
            .map(|meeting| MeetingView {
                id: meeting.id.clone(),
                date: date_only(&meeting.date),
                venue_code: meeting.venue_code.clone(),
                venue: venue_name(&meeting.venue_code),
                status: meeting.status.clone(),
                day_of_week: String::new(),
                meeting_type: String::new(),
                country: "Hong Kong".to_string(),
                total_races: meeting.races.len(),
                current_race: None,
                races: meeting
                    .races
                    .iter()
                    .map(|race| RaceView {
                        no: race.no,
                        post_time: race.post_time.as_deref().map(hhmm).unwrap_or_default(),
                        status: race.status.clone(),
                        distance: String::new(),
                        class: String::new(),
                        race_name: String::new(),
                        going: String::new(),
                        track: String::new(),
                        course: String::new(),
                        field_size: race.wagering_field_size,
                    })
                    .collect(),
            })
            .collect()
    }
}

impl Meeting {
    pub fn view(&self) -> MeetingView {
        MeetingView {
            id: self.id.clone(),
            date: date_only(&self.date),
            venue_code: self.venue_code.clone(),
            venue: venue_name(&self.venue_code),
            status: self.status.clone(),
            day_of_week: self.date_of_week.clone().unwrap_or_default(),
            meeting_type: self.meeting_type.clone().unwrap_or_default(),
            country: self
                .country
                .first()
                .map(|country| {
                    if country.nameen.is_empty() {
                        country.code.clone()
                    } else {
                        country.nameen.clone()
                    }
                })
                .unwrap_or_else(|| "Hong Kong".to_string()),
            total_races: self
                .total_number_of_race
                .map(|count| count as usize)
                .unwrap_or(self.races.len()),
            current_race: self.current_number_of_race,
            races: self.races.iter().map(Race::view).collect(),
        }
    }

    pub fn race(&self, no: i64) -> Option<&Race> {
        self.races.iter().find(|race| race.no == no)
    }
}

impl Race {
    pub fn view(&self) -> RaceView {
        let class_text = if !self.race_class_en.is_empty() {
            self.race_class_en.clone()
        } else {
            self.cla_code.clone()
        };
        RaceView {
            no: self.no,
            post_time: self.post_time.as_deref().map(hhmm).unwrap_or_default(),
            status: self.status.clone(),
            distance: self.distance.as_ref().map(scalar).unwrap_or_default(),
            class: class_text,
            race_name: self.race_name_en.clone(),
            going: self.going_en.clone(),
            track: self
                .race_track
                .as_ref()
                .map(|track| track.description_en.clone())
                .unwrap_or_default(),
            course: self
                .race_course
                .as_ref()
                .map(|course| course.description_en.clone())
                .unwrap_or_default(),
            field_size: self.wagering_field_size,
        }
    }

    pub fn card(&self, meeting: &Meeting) -> RaceCardView {
        RaceCardView {
            meeting_id: meeting.id.clone(),
            date: date_only(&meeting.date),
            venue_code: meeting.venue_code.clone(),
            venue: venue_name(&meeting.venue_code),
            race: self.view(),
            runners: self.runners.iter().map(Runner::view).collect(),
        }
    }
}

impl Runner {
    pub fn view(&self) -> RunnerView {
        RunnerView {
            no: self.no,
            horse_no: self.no,
            horse: self.name_en.clone(),
            horse_zh: self.name_ch.clone(),
            horse_id: self
                .horse
                .as_ref()
                .map(|horse| {
                    if horse.id.is_empty() {
                        horse.code.clone()
                    } else {
                        horse.id.clone()
                    }
                })
                .unwrap_or_default(),
            jockey: self
                .jockey
                .as_ref()
                .map(|person| person.name_en.clone())
                .unwrap_or_default(),
            jockey_code: self
                .jockey
                .as_ref()
                .map(|person| person.code.clone())
                .unwrap_or_default(),
            trainer: self
                .trainer
                .as_ref()
                .map(|person| person.name_en.clone())
                .unwrap_or_default(),
            trainer_code: self
                .trainer
                .as_ref()
                .map(|person| person.code.clone())
                .unwrap_or_default(),
            draw: text(&self.barrier_draw_number),
            weight: text(&self.handicap_weight),
            rating: text(&self.current_rating),
            last_6_runs: self.last6run.clone().unwrap_or_default(),
            gear: self.gear_info.clone().unwrap_or_default(),
            status: self.status.clone(),
            win_odds: text(&self.win_odds),
            final_position: text(&self.final_position),
        }
    }
}

impl Draw {
    pub fn view(&self) -> MarkSixDrawView {
        let result = self.draw_result.as_ref();
        let pool = self.lottery_pool.as_ref();
        let snowball = if self.snowball_name_en.is_empty() {
            String::new()
        } else {
            self.snowball_name_en.clone()
        };
        let mut prizes: Vec<MarkSixPrizeView> = pool
            .map(|pool| {
                pool.lottery_prizes
                    .iter()
                    .map(|prize| MarkSixPrizeView {
                        division: prize.kind,
                        winning_units: text(&prize.winning_unit),
                        dividend: text(&prize.dividend),
                    })
                    .collect()
            })
            .unwrap_or_default();
        prizes.sort_by_key(|prize| prize.division);
        MarkSixDrawView {
            draw_id: self.id.clone(),
            year: self.year.clone(),
            draw_no: self.no,
            draw_date: self.draw_date.as_deref().map(date_only).unwrap_or_default(),
            status: self.status.clone(),
            snowball,
            numbers: result
                .map(|result| result.drawn_no.iter().map(|n| *n as u8).collect())
                .unwrap_or_default(),
            extra: result.and_then(|result| result.x_drawn_no.map(|n| n as u8)),
            jackpot: pool.map(|pool| text(&pool.jackpot)).unwrap_or_default(),
            turnover: pool
                .map(|pool| text(&pool.total_investment))
                .unwrap_or_default(),
            unit_bet: pool.map(|pool| text(&pool.unit_bet)).unwrap_or_default(),
            estimated_first_prize: pool
                .map(|pool| {
                    let estimated = text(&pool.estimated_prize);
                    if estimated.is_empty() {
                        text(&pool.derived_first_prize_div)
                    } else {
                        estimated
                    }
                })
                .unwrap_or_default(),
            prizes,
        }
    }
}

impl OddsData {
    pub fn view(&self, date: NaiveDate, venue: &str, race_no: i64) -> OddsView {
        let pools = self
            .race_meetings
            .iter()
            .flat_map(|meeting| meeting.pm_pools.iter())
            .filter(|pool| pool.odds_type != "ALUP")
            .map(|pool| OddsPoolView {
                pool: pool.odds_type.clone(),
                name: pool
                    .name_en
                    .clone()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| pool.odds_type.clone()),
                status: pool.status.clone(),
                sell_status: pool.sell_status.clone(),
                last_update: pool
                    .last_update_time
                    .as_deref()
                    .map(hhmm)
                    .unwrap_or_default(),
                legs: pool
                    .leg
                    .as_ref()
                    .map(|leg| leg.races.clone())
                    .unwrap_or_default(),
                odds: pool
                    .odds_nodes
                    .iter()
                    .map(|node| OddsEntry {
                        combination: node.comb_string.clone(),
                        odds: text(&node.odds_value),
                        hot_favourite: node.hot_favourite.unwrap_or(false),
                        bankers: node
                            .banker_odds
                            .iter()
                            .map(|banker| OddsEntry {
                                combination: banker.comb_string.clone(),
                                odds: text(&banker.odds_value),
                                hot_favourite: false,
                                bankers: Vec::new(),
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect();
        OddsView {
            date: date.to_string(),
            venue_code: venue.to_uppercase(),
            venue: venue_name(&venue.to_uppercase()),
            race_no,
            pools,
        }
    }
}

/// Frequency statistics across Mark Six draws.
pub fn draw_label(draw: &Draw) -> String {
    format!("{}/{:03}", draw.year, draw.no)
}

pub fn mark_six_stats(draws: &[Draw]) -> MarkSixStatsView {
    let mut counts = [0u64; 50];
    let mut last_seen: [Option<(i64, i64)>; 50] = [None; 50];
    let mut odd = 0u64;
    let mut even = 0u64;
    let mut sum_total = 0u64;
    let mut analysed = 0usize;
    let mut draw_marks: Vec<(i64, i64)> = Vec::new();
    for draw in draws {
        let Some(result) = draw.draw_result.as_ref() else {
            continue;
        };
        if result.drawn_no.is_empty() {
            continue;
        }
        let year = draw.year.parse::<i64>().unwrap_or(0);
        let mark = (year, draw.no);
        analysed += 1;
        draw_marks.push(mark);
        sum_total += result.drawn_no.iter().map(|n| *n as u64).sum::<u64>();
        for number in &result.drawn_no {
            if (1..=49).contains(number) {
                counts[*number as usize] += 1;
                let seen = &mut last_seen[*number as usize];
                if seen.is_none_or(|previous| mark > previous) {
                    *seen = Some(mark);
                }
                if number % 2 == 0 {
                    even += 1;
                } else {
                    odd += 1;
                }
            }
        }
    }
    let total_numbers = (analysed * 6).max(1) as f64;
    let mut numbers: Vec<NumberStat> = (1..=49)
        .map(|number| NumberStat {
            number: number as u8,
            count: counts[number as usize],
            last_seen_draw: last_seen[number as usize].map(|(year, no)| format!("{year}/{no:03}")),
            percentage: (counts[number as usize] as f64 / total_numbers * 1000.0).round() / 10.0,
        })
        .collect();
    let mut hot = numbers.clone();
    hot.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| right.last_seen_draw.cmp(&left.last_seen_draw))
    });
    hot.truncate(10);
    let mut cold = numbers.clone();
    cold.sort_by(|left, right| {
        left.count
            .cmp(&right.count)
            .then_with(|| left.last_seen_draw.cmp(&right.last_seen_draw))
    });
    cold.truncate(10);
    numbers.sort_by_key(|stat| stat.number);
    MarkSixStatsView {
        draws_analysed: analysed,
        from_draw: draw_marks
            .iter()
            .min()
            .map(|(year, no)| format!("{year}/{no:03}")),
        to_draw: draw_marks
            .iter()
            .max()
            .map(|(year, no)| format!("{year}/{no:03}")),
        hot,
        cold,
        numbers,
        odd_even: OddEven { odd, even },
        average_sum: if analysed == 0 {
            0.0
        } else {
            (sum_total as f64 / analysed as f64 * 100.0).round() / 100.0
        },
    }
}

// ---------------------------------------------------------------------------
// racing.hkjc.com results HTML
// ---------------------------------------------------------------------------

fn normalize(value: &str) -> String {
    value
        .replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn table_rows(table: &scraper::ElementRef) -> Vec<Vec<String>> {
    let row_selector = Selector::parse("tr").expect("valid selector");
    let cell_selector = Selector::parse("td, th").expect("valid selector");
    table
        .select(&row_selector)
        .map(|row| {
            row.select(&cell_selector)
                .map(|cell| normalize(&cell.text().collect::<Vec<_>>().join(" ")))
                .collect()
        })
        .collect()
}

fn squash(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn find_column_squashed(header: &[String], needle: &str) -> Option<usize> {
    let needle = squash(needle);
    header
        .iter()
        .position(|cell| squash(cell) == needle)
        .or_else(|| {
            header
                .iter()
                .position(|cell| squash(cell).contains(&needle))
        })
}

fn find_column(header: &[String], needle: &str) -> Option<usize> {
    let needle = needle.to_lowercase();
    header
        .iter()
        .position(|cell| cell.to_lowercase() == needle)
        .or_else(|| {
            header
                .iter()
                .position(|cell| cell.to_lowercase().contains(&needle))
        })
}

pub fn parse_results_page(html: &str, race_no: i64) -> Result<RaceResultView> {
    let document = Html::parse_document(html);
    let table_selector = Selector::parse("table").expect("valid selector");

    let mut placings = Vec::new();
    let mut dividends = Vec::new();
    let mut incidents = Vec::new();
    let mut header_cells: Vec<String> = Vec::new();

    for table in document.select(&table_selector) {
        // Keep only innermost tables so layout wrappers do not duplicate rows.
        if table.select(&table_selector).next().is_some() {
            continue;
        }
        let rows = table_rows(&table);
        if rows.is_empty() {
            continue;
        }
        let table_text = rows
            .iter()
            .flat_map(|row| row.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        let header_index = |needles: &[&str]| {
            rows.iter().position(|row| {
                let text = row.join(" ").to_lowercase();
                needles
                    .iter()
                    .all(|needle| text.contains(&needle.to_lowercase()))
            })
        };
        if let Some(index) = header_index(&["Pla.", "Horse No.", "Jockey"]) {
            let header = &rows[index];
            let place = find_column(header, "Pla.");
            let horse_no = find_column(header, "Horse No.");
            let horse = find_column(header, "Horse");
            let jockey = find_column(header, "Jockey");
            let trainer = find_column(header, "Trainer");
            let weight = find_column(header, "Act. Wt.");
            let declared = find_column(header, "Declar. Horse Wt.");
            let draw = find_column(header, "Dr.");
            let lbw = find_column(header, "LBW");
            let position =
                find_column(header, "Running Position").or_else(|| find_column(header, "Running"));
            let finish = find_column(header, "Finish Time");
            let odds = find_column(header, "Win Odds");
            for row in rows.iter().skip(index + 1) {
                let get = |column: Option<usize>| {
                    column
                        .and_then(|index| row.get(index))
                        .cloned()
                        .unwrap_or_default()
                };
                if row.iter().all(|cell| cell.is_empty()) {
                    continue;
                }
                let place_value = get(place);
                if place_value.is_empty() {
                    continue;
                }
                placings.push(PlacingView {
                    place: place_value,
                    horse_no: get(horse_no),
                    horse: get(horse),
                    jockey: get(jockey),
                    trainer: get(trainer),
                    actual_weight: get(weight),
                    declared_weight: get(declared),
                    draw: get(draw),
                    lengths_behind: get(lbw),
                    running_position: get(position),
                    finish_time: get(finish),
                    win_odds: get(odds),
                });
            }
        } else if let Some(index) = header_index(&["Winning Combination"]) {
            let mut current_pool = String::new();
            for row in rows.iter().skip(index + 1) {
                if row.len() < 3 {
                    continue;
                }
                let mut pool = row[0].clone();
                if pool.is_empty() {
                    pool = current_pool.clone();
                } else {
                    current_pool = pool.clone();
                }
                let combination = row[1].clone();
                let dividend = row[2].clone();
                if pool.is_empty() || combination.is_empty() {
                    continue;
                }
                dividends.push(DividendView {
                    pool,
                    combination,
                    dividend,
                });
            }
        } else if let Some(index) = header_index(&["Incident", "Horse No."]) {
            let header = &rows[index];
            let place = find_column(header, "Pla.");
            let horse_no = find_column(header, "Horse No.");
            let horse = find_column(header, "Horse");
            let incident = find_column(header, "Incident");
            for row in rows.iter().skip(index + 1) {
                let get = |column: Option<usize>| {
                    column
                        .and_then(|index| row.get(index))
                        .cloned()
                        .unwrap_or_default()
                };
                let text = get(incident);
                if text.is_empty() {
                    continue;
                }
                incidents.push(IncidentView {
                    place: get(place),
                    horse_no: get(horse_no),
                    horse: get(horse),
                    incident: text,
                });
            }
        } else if table_text.contains("Class")
            && table_text.contains("Going")
            && table_text.contains("RACE ")
        {
            for row in rows {
                for cell in row {
                    if !cell.is_empty() {
                        header_cells.push(cell);
                    }
                }
            }
        }
    }

    let header_blob = header_cells.join(" | ");
    let race_id = extract_race_id(&header_blob, race_no);
    let class_full = header_cells
        .iter()
        .find(|cell| cell.starts_with("Class"))
        .cloned()
        .unwrap_or_default();
    let distance = class_full
        .split(" - ")
        .find(|part| part.ends_with('M'))
        .map(ToString::to_string)
        .unwrap_or_default();
    let class = class_full
        .split(" - ")
        .next()
        .unwrap_or(&class_full)
        .to_string();
    let going = value_after(&header_cells, "going");
    let course = value_after(&header_cells, "course");
    let prize = header_cells
        .iter()
        .find(|cell| cell.starts_with("HK$"))
        .cloned()
        .unwrap_or_default();
    let time_line = header_cells
        .iter()
        .find(|cell| cell.starts_with("Time"))
        .cloned()
        .unwrap_or_default();
    let finish_time = placings
        .first()
        .map(|placing| placing.finish_time.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            time_line
                .rsplit('(')
                .next()
                .map(|value| value.trim_end_matches(')').to_string())
                .unwrap_or_default()
        });
    let race_name = header_cells
        .iter()
        .filter(|cell| {
            let upper = cell.to_uppercase();
            !cell.contains(':')
                && !cell.starts_with("Class")
                && !cell.contains("HK$")
                && !upper.starts_with("RACE ")
                && !upper.contains("COURSE")
                && !upper.contains("TURF")
                && !upper.contains("ALL WEATHER")
                && !upper.contains("DIRT")
                && cell
                    .chars()
                    .any(|character| character.is_ascii_alphabetic())
                && cell.len() > 6
        })
        .max_by_key(|cell| cell.len())
        .cloned()
        .unwrap_or_default();

    Ok(RaceResultView {
        date: String::new(),
        venue: String::new(),
        race_no,
        race_id,
        race_name,
        class,
        distance,
        going,
        course,
        prize,
        finish_time,
        placings,
        dividends,
        incidents,
    })
}

/// The first non-empty cell after the one containing `needle`.
fn value_after(cells: &[String], needle: &str) -> String {
    let needle = needle.to_lowercase();
    if let Some(index) = cells
        .iter()
        .position(|cell| cell.to_lowercase().contains(&needle))
    {
        for cell in &cells[index + 1..] {
            if !cell.is_empty() {
                return cell.clone();
            }
        }
    }
    String::new()
}

fn extract_race_id(blob: &str, race_no: i64) -> String {
    let needle = format!("RACE {race_no} (");
    if let Some(start) = blob.find(&needle) {
        let rest = &blob[start + needle.len()..];
        let digits: String = rest
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect();
        return digits;
    }
    String::new()
}

// ---------------------------------------------------------------------------
// Jockey / trainer / horse profile pages
// ---------------------------------------------------------------------------

fn profile_tables(html: &str) -> (Html, Vec<Vec<Vec<String>>>) {
    let document = Html::parse_document(html);
    let table_selector = Selector::parse("table").expect("valid selector");
    let tables = document
        .select(&table_selector)
        .map(|table| table_rows(&table))
        .filter(|rows| !rows.is_empty())
        .collect();
    (document, tables)
}

fn cell_count(table: &[Vec<String>]) -> usize {
    table.iter().map(|row| row.len()).sum()
}

fn table_text(table: &[Vec<String>]) -> String {
    table
        .iter()
        .flat_map(|row| row.iter())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
}

fn labelled(cells: &[String], label: &str) -> String {
    let label = label.to_lowercase();
    for (index, cell) in cells.iter().enumerate() {
        let trimmed = cell.trim();
        if trimmed.to_lowercase().starts_with(&label) {
            let rest = trimmed[label.len()..].trim_start_matches(':').trim();
            if !rest.is_empty() {
                return rest.to_string();
            }
            if let Some(next) = cells[index + 1..].iter().find(|cell| !cell.is_empty()) {
                return next.trim().to_string();
            }
        }
    }
    String::new()
}

fn parse_pairs(cells: &[String]) -> Vec<ProfileStat> {
    let mut out: Vec<ProfileStat> = Vec::new();
    for (index, cell) in cells.iter().enumerate() {
        let trimmed = cell.trim();
        let Some(rest) = trimmed.strip_prefix(':') else {
            continue;
        };
        let label = cells[..index]
            .iter()
            .rev()
            .find(|cell| !cell.is_empty() && cell.trim() != ":")
            .map(|cell| cell.trim().trim_end_matches(':').trim().to_string());
        let value = if rest.trim().is_empty() {
            cells[index + 1..]
                .iter()
                .find(|cell| !cell.is_empty() && cell.trim() != ":")
                .map(|cell| cell.trim().to_string())
        } else {
            Some(rest.trim().to_string())
        };
        if let (Some(label), Some(value)) = (label, value)
            && !label.is_empty()
            && !value.is_empty()
            && !out.iter().any(|stat| stat.label == label)
        {
            out.push(ProfileStat { label, value });
        }
    }
    out
}

fn parse_form(rows: &[Vec<String>]) -> Vec<HorseRunView> {
    let Some(header_index) = rows.iter().position(|row| {
        let text = squash(&row.join(" "));
        text.contains("raceindex") && text.contains("pla.") && text.contains("date")
    }) else {
        return Vec::new();
    };
    let header = &rows[header_index];
    let get = |row: &[String], column: Option<usize>| {
        column
            .and_then(|index| row.get(index))
            .cloned()
            .unwrap_or_default()
    };
    let index_col = find_column_squashed(header, "RaceIndex");
    let place_col = find_column_squashed(header, "Pla.");
    let date_col = find_column_squashed(header, "Date");
    let track_col = find_column_squashed(header, "RC/Track");
    let distance_col = find_column_squashed(header, "Dist.");
    let going_col = find_column_squashed(header, "G");
    let class_col = find_column_squashed(header, "RaceClass");
    let draw_col = find_column_squashed(header, "Dr.");
    let rating_col = find_column_squashed(header, "Rtg.");
    let trainer_col = find_column_squashed(header, "Trainer");
    let jockey_col = find_column_squashed(header, "Jockey");
    let lbw_col = find_column_squashed(header, "LBW");
    let odds_col = find_column_squashed(header, "Win Odds");
    let weight_col = find_column_squashed(header, "Act.Wt.");
    let finish_col = find_column_squashed(header, "Finish Time");
    let mut season = String::new();
    let mut out = Vec::new();
    for row in &rows[header_index + 1..] {
        let filled: Vec<&String> = row.iter().filter(|cell| !cell.is_empty()).collect();
        if filled.len() == 1 && filled[0].contains("Season") {
            season = filled[0].clone();
            continue;
        }
        let place = get(row, place_col);
        if place.is_empty() {
            continue;
        }
        out.push(HorseRunView {
            season: season.clone(),
            index: get(row, index_col),
            place,
            date: get(row, date_col),
            track: get(row, track_col),
            distance: get(row, distance_col),
            going: get(row, going_col),
            class: get(row, class_col),
            draw: get(row, draw_col),
            rating: get(row, rating_col),
            trainer: get(row, trainer_col),
            jockey: get(row, jockey_col),
            lengths_behind: get(row, lbw_col),
            win_odds: get(row, odds_col),
            weight: get(row, weight_col),
            finish_time: get(row, finish_col),
        });
    }
    out
}

pub fn parse_profile(html: &str, kind: ProfileKind, id: &str, source: &str) -> ProfileView {
    let (document, tables) = profile_tables(html);
    let title = document
        .select(&Selector::parse("title").expect("valid selector"))
        .next()
        .map(|element| normalize(&element.text().collect::<Vec<_>>().join(" ")))
        .unwrap_or_default();
    let all_cells: Vec<String> = tables
        .iter()
        .flat_map(|table| table.iter().flat_map(|row| row.iter()))
        .cloned()
        .collect();
    let bio = tables
        .iter()
        .filter(|table| table_text(table).contains("Age"))
        .min_by_key(|table| cell_count(table));
    let bio_cells: Vec<String> = bio
        .map(|table| table.iter().flat_map(|row| row.iter()).cloned().collect())
        .unwrap_or_default();

    let mut name = String::new();
    if kind == ProfileKind::Horse {
        name = title.split(" - ").next().unwrap_or("").trim().to_string();
        if name.eq_ignore_ascii_case("The Hong Kong Jockey Club") {
            name.clear();
        }
        if name.is_empty()
            && let Some(cell) = all_cells.iter().find(|cell| cell.contains('('))
        {
            name = cell.split('(').next().unwrap_or("").trim().to_string();
        }
    } else if let Some(table) = bio
        && let Some(cell) = table.first().and_then(|row| row.first())
        && !cell.is_empty()
        && !cell.contains(':')
        && cell.len() < 60
    {
        name = cell.clone();
    }
    if name.is_empty() {
        name = title.split(" - ").next().unwrap_or("").trim().to_string();
    }

    let age = labelled(&bio_cells, "Age");
    let background = labelled(&bio_cells, "Background");
    let season = all_cells
        .iter()
        .find(|cell| cell.contains("Season") && cell.contains("Up to"))
        .cloned()
        .unwrap_or_default();

    let mut stats: Vec<ProfileStat> = Vec::new();
    for table in &tables {
        let text = table
            .iter()
            .flat_map(|row| row.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        let wanted = if kind == ProfileKind::Horse {
            text.contains("Country of Origin")
                || (text.contains("Sire") && text.contains("Trainer"))
        } else {
            text.contains("No. of Wins") || text.contains("Total Runners")
        };
        if !wanted || cell_count(table) > 120 {
            continue;
        }
        let cells: Vec<String> = table.iter().flat_map(|row| row.iter()).cloned().collect();
        for stat in parse_pairs(&cells) {
            if !stats.iter().any(|existing| existing.label == stat.label) {
                stats.push(stat);
            }
        }
    }

    let form = if kind == ProfileKind::Horse {
        let candidates: Vec<_> = tables
            .iter()
            .filter(|table| {
                table.iter().any(|row| {
                    let text = squash(&row.join(" "));
                    text.contains("raceindex") && text.contains("pla.")
                })
            })
            .collect();
        candidates
            .into_iter()
            .max_by_key(|table| table.len())
            .map(|table| parse_form(table))
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    ProfileView {
        kind: kind.as_str().to_string(),
        id: id.to_string(),
        name,
        age,
        background,
        season,
        stats,
        form,
        source: source.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn venue_names() {
        assert_eq!(venue_name("ST"), "Sha Tin");
        assert_eq!(venue_name("HV"), "Happy Valley");
        assert_eq!(venue_name("S1"), "Simulcast S1");
    }

    #[test]
    fn parses_result_tables() {
        let html = r#"<html><body>
        <table><tr><td>RACE 1 (198)</td><td>Class 5 - 2200M - (40-0)</td><td>Going :</td><td>GOOD</td></tr></table>
        <table>
          <tr><th>Pla.</th><th>Horse No.</th><th>Horse</th><th>Jockey</th><th>Trainer</th>
              <th>Act. Wt.</th><th>Declar. Horse Wt.</th><th>Dr.</th><th>LBW</th>
              <th>RunningPosition</th><th>Finish Time</th><th>Win Odds</th></tr>
          <tr><td>1</td><td>1</td><td>STAR BROSE (J463)</td><td>Z Purton</td><td>D A Hayes</td>
              <td>135</td><td>1219</td><td>5</td><td>---</td><td>8 7 6 6 5 1</td><td>2:17.62</td><td>3.1</td></tr>
        </table>
        <table>
          <tr><th>Pool</th><th>Winning Combination</th><th>Dividend (HK$)</th></tr>
          <tr><td>WIN</td><td>1</td><td>31.50</td></tr>
          <tr><td>PLACE</td><td>1</td><td>14.00</td></tr>
          <tr><td></td><td>9</td><td>32.00</td></tr>
        </table>
        </body></html>"#;
        let parsed = parse_results_page(html, 1).unwrap();
        assert_eq!(parsed.race_id, "198");
        assert_eq!(parsed.distance, "2200M");
        assert_eq!(parsed.placings.len(), 1);
        assert_eq!(parsed.placings[0].horse, "STAR BROSE (J463)");
        assert_eq!(parsed.dividends.len(), 3);
        assert_eq!(parsed.dividends[2].pool, "PLACE");
    }

    #[test]
    fn parses_profile_stats() {
        let cells: Vec<String> = [
            "Nationality",
            ": AUS",
            "No. of Wins",
            ": 34",
            "Win %",
            ":",
            "21.66%",
        ]
        .iter()
        .map(|cell| cell.to_string())
        .collect();
        let stats = parse_pairs(&cells);
        assert_eq!(stats.len(), 3);
        assert_eq!(stats[0].label, "Nationality");
        assert_eq!(stats[0].value, "AUS");
        assert_eq!(stats[1].value, "34");
        assert_eq!(stats[2].label, "Win %");
        assert_eq!(stats[2].value, "21.66%");
    }

    #[test]
    fn parses_profile_page() {
        let html = r#"<html><head><title>Zac Purton - Jockey</title></head><body>
        <table><tr><td>Zac Purton</td><td>Age: 43</td><td>Background: Test bio</td></tr></table>
        <table>
          <tr><td>26/27 Season Up to Race Meeting of 18/11/2026</td></tr>
          <tr><td>No. of Wins</td><td>: 34</td><td>Total Rides</td><td>: 157</td></tr>
        </table>
        </body></html>"#;
        let profile = parse_profile(html, ProfileKind::Jockey, "PZ", "test");
        assert_eq!(profile.name, "Zac Purton");
        assert_eq!(profile.age, "43");
        assert_eq!(profile.background, "Test bio");
        assert!(profile.season.contains("26/27 Season"));
        assert_eq!(profile.stats.len(), 2);
        assert_eq!(profile.stats[0].label, "No. of Wins");
        assert_eq!(profile.stats[0].value, "34");
    }

    #[test]
    fn parses_spaced_form_header() {
        let row = |cells: &[&str]| {
            cells
                .iter()
                .map(|cell| cell.to_string())
                .collect::<Vec<_>>()
        };
        let rows = vec![
            row(&[
                "Race Index",
                "Pla.",
                "Date",
                "RC /Track/ Course",
                "Dist.",
                "G",
                "Race Class",
                "Dr.",
                "Rtg.",
                "Trainer",
                "Jockey",
                "LBW",
                "Win Odds",
                "Act. Wt.",
                "Running Position",
                "Finish Time",
            ]),
            row(&["26/27 Season"]),
            row(&[
                "150",
                "12",
                "01/11/26",
                "HV / Turf / \"C+3\"",
                "1200",
                "G",
                "4",
                "8",
                "43",
                "K H Ting",
                "Y L Chung",
                "13-1/4",
                "74",
                "116",
                "2 2 12",
                "1.12.30",
            ]),
        ];
        let form = parse_form(&rows);
        assert_eq!(form.len(), 1);
        assert_eq!(form[0].place, "12");
        assert_eq!(form[0].date, "01/11/26");
        assert_eq!(form[0].track, "HV / Turf / \"C+3\"");
        assert_eq!(form[0].jockey, "Y L Chung");
        assert_eq!(form[0].season, "26/27 Season");
    }

    #[test]
    fn parses_fixture_calendar_cells() {
        let html = r#"<table>
          <tr><td><span class="f_fl f_fs14">2</span><span class="f_fr">
            <img src="hv.gif" alt="HV" /><img src="night.gif" alt="N" /></span></td></tr>
          <tr><td><span class="f_fl f_fs14">25</span><span class="f_fr">
            <img src="st.gif" alt="ST" /><img src="day.gif" alt="D" /></span></td></tr>
        </table>"#;
        let fixtures = parse_fixture_month(html, 2026, 12);
        assert_eq!(fixtures.len(), 2);
        assert_eq!(fixtures[0].date, "2026-12-02");
        assert_eq!(fixtures[0].venue_code, "HV");
        assert_eq!(fixtures[1].date, "2026-12-25");
        assert_eq!(fixtures[1].venue_code, "ST");
    }
}
