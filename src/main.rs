//! `hkjc` — Hong Kong Jockey Club horse racing and Mark Six CLI.

use anyhow::{Result, bail};
use chrono::{Days, NaiveDate};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use hkjc::api::{self, Client};
use hkjc::model::ProfileKind;

mod update;

#[derive(Parser)]
#[command(
    name = "hkjc",
    version,
    about = "Hong Kong Jockey Club horse racing and Mark Six — CLI and MCP server",
    arg_required_else_help = true
)]
struct Cli {
    /// Print machine-readable JSON
    #[arg(long, global = true, conflicts_with = "csv")]
    json: bool,

    /// Print RFC 4180 CSV
    #[arg(long, global = true)]
    csv: bool,

    /// Local history database (default: XDG data directory)
    #[arg(long, global = true, value_name = "PATH")]
    db: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Current race meeting, races and post times
    #[command(alias = "races")]
    Meetings {
        /// Meeting date, YYYY-MM-DD (default: all active meetings)
        #[arg(long)]
        date: Option<String>,
        /// Venue code: ST, HV, CH or S1–S9
        #[arg(long, short)]
        venue: Option<String>,
    },
    /// Runners, jockeys, trainers and barriers for one race
    Card {
        /// Race number (default 1)
        #[arg(long, short, default_value_t = 1)]
        race: i64,
        /// Meeting date, YYYY-MM-DD
        #[arg(long)]
        date: Option<String>,
        /// Venue code: ST, HV, CH or S1–S9
        #[arg(long, short)]
        venue: Option<String>,
    },
    /// Live dividends odds for one race
    Odds {
        /// Race number (default 1)
        #[arg(long, short, default_value_t = 1)]
        race: i64,
        /// Comma-separated pools: WIN,PLA,QIN,QPL,FCT,TCE,TRI,FF,QTT,CWA,IWN
        #[arg(long)]
        types: Option<String>,
        /// Save this odds board to the local history database
        #[arg(long)]
        record: bool,
        /// Meeting date, YYYY-MM-DD
        #[arg(long)]
        date: Option<String>,
        /// Venue code: ST, HV, CH or S1–S9
        #[arg(long, short)]
        venue: Option<String>,
    },
    /// Odds snapshots recorded in the local history database
    History {
        /// Meeting date, YYYY-MM-DD (default today)
        #[arg(long)]
        date: Option<String>,
        /// Venue code: ST, HV or CH
        #[arg(long, short, default_value = "ST")]
        venue: String,
        /// Race number (default 1)
        #[arg(long, short, default_value_t = 1)]
        race: i64,
        /// Only this pool
        #[arg(long)]
        pool: Option<String>,
        /// Maximum combinations (0 means all)
        #[arg(long, default_value_t = 40)]
        limit: usize,
    },
    /// Historical race result, placings and dividends
    Results {
        /// Race date, YYYY-MM-DD (default today)
        #[arg(long)]
        date: Option<String>,
        /// Venue code: ST or HV (default: try Sha Tin, then Happy Valley)
        #[arg(long, short)]
        venue: Option<String>,
        /// Race number (default 1)
        #[arg(long, short, default_value_t = 1)]
        race: i64,
    },
    /// Mark Six draw results
    Marksix {
        /// Number of most recent draws (default 10)
        #[arg(long, short, default_value_t = 10)]
        draws: i64,
        /// First draw date, YYYY-MM-DD
        #[arg(long)]
        from: Option<String>,
        /// Last draw date, YYYY-MM-DD
        #[arg(long)]
        to: Option<String>,
    },
    /// The latest Mark Six draw with the prize breakdown
    MarksixLatest,
    /// Mark Six number frequency from recent draws
    MarksixStats {
        /// Number of recent draws to analyse (default 100)
        #[arg(long, short, default_value_t = 100)]
        draws: i64,
    },
    /// Jockey or trainer championship standings
    Standings {
        /// Which championship table
        #[arg(value_enum)]
        kind: StandingArg,
        /// Season code such as 25/26 (default: current season)
        #[arg(long)]
        season: Option<String>,
        /// Racecourse: ST or HV (default: all)
        #[arg(long, short)]
        venue: Option<String>,
    },
    /// Barrier trial results for one date
    Trials {
        /// Trial date, YYYY-MM-DD (default today)
        #[arg(long)]
        date: Option<String>,
    },
    /// Sectional times for one race
    Sectionals {
        /// Race date, YYYY-MM-DD (default today)
        #[arg(long)]
        date: Option<String>,
        /// Race number (default 1)
        #[arg(long, short, default_value_t = 1)]
        race: i64,
    },
    /// Pace map from the sectional running positions of one race
    Pace {
        /// Race date, YYYY-MM-DD (default today)
        #[arg(long)]
        date: Option<String>,
        /// Race number (default 1)
        #[arg(long, short, default_value_t = 1)]
        race: i64,
    },
    /// Check a Mark Six ticket against a draw
    MarksixCheck {
        /// Six numbers, separated by commas or spaces
        numbers: String,
        /// Draw number (default: the latest draw)
        #[arg(long)]
        draw: Option<i64>,
    },
    /// Season fixture list of race meetings
    Fixtures {
        /// First date, YYYY-MM-DD (default today, or the season start with --all)
        #[arg(long)]
        from: Option<String>,
        /// Last date, YYYY-MM-DD (default 90 days after --from)
        #[arg(long)]
        to: Option<String>,
        /// Venue code: ST, HV, CH or S1–S9
        #[arg(long, short)]
        venue: Option<String>,
        /// The whole published season
        #[arg(long)]
        all: bool,
        /// Print an iCalendar feed instead of a table
        #[arg(long)]
        ics: bool,
        /// Only the next meeting
        #[arg(long)]
        next: bool,
    },
    /// Football matches with HAD odds
    Football {
        /// First match date, YYYY-MM-DD
        #[arg(long)]
        from: Option<String>,
        /// Last match date, YYYY-MM-DD
        #[arg(long)]
        to: Option<String>,
        /// Tournament name or code substring
        #[arg(long, short)]
        tournament: Option<String>,
        /// Maximum matches (default 50; 0 means all)
        #[arg(long, short, default_value_t = 50)]
        limit: usize,
    },
    /// Full odds board for one football match
    FootballMatch {
        /// Match id from `football`
        id: String,
        /// Comma-separated pools: HAD,HHA,CRS,TTG,HIL,CHL,HDC
        #[arg(long)]
        types: Option<String>,
    },
    /// Official jockey, trainer or horse profile
    Profile {
        /// Profile kind
        #[arg(value_enum)]
        kind: ProfileArg,
        /// Profile id from `lookup` (jockey code, trainer code or horse id)
        id: String,
    },
    /// Find a jockey, trainer or horse in the current meeting
    Lookup {
        /// Name or part of a name, English or Chinese
        query: String,
    },
    /// Update this binary using the method it was installed with
    Update {
        /// Only report the current version, latest release and install method
        #[arg(long)]
        check: bool,
        /// Reinstall even when the current version is up to date
        #[arg(long)]
        force: bool,
    },
    /// Poll a meeting and report odds moves and scratchings
    Watch {
        /// Meeting date, YYYY-MM-DD (default today)
        #[arg(long)]
        date: Option<String>,
        /// Venue code: ST, HV or CH
        #[arg(long, short)]
        venue: Option<String>,
        /// Only this race (default: every race on the card)
        #[arg(long, short)]
        race: Option<i64>,
        /// Seconds between polls (minimum 5)
        #[arg(long, default_value_t = 60)]
        interval: u64,
        /// Stop after this many polls (0 means until interrupted)
        #[arg(long, default_value_t = 0)]
        count: u32,
        /// POST each change as JSON to this URL
        #[arg(long)]
        webhook: Option<String>,
        /// Send changes to Telegram as BOT_TOKEN:CHAT_ID
        #[arg(long)]
        telegram: Option<String>,
        /// Save every poll to the local history database
        #[arg(long)]
        record: bool,
    },
    /// Print a shell completion script
    Completions {
        /// Target shell
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Print the roff manual page
    Man,
    /// Check HKJC endpoint health and latency
    Doctor,
    /// Run the MCP server over stdio
    Mcp,
}

#[derive(Clone, Copy, ValueEnum)]
enum StandingArg {
    Jockey,
    Trainer,
}

impl StandingArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::Jockey => "jockey",
            Self::Trainer => "trainer",
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum ProfileArg {
    Jockey,
    Trainer,
    Horse,
}

impl From<ProfileArg> for ProfileKind {
    fn from(value: ProfileArg) -> Self {
        match value {
            ProfileArg::Jockey => Self::Jockey,
            ProfileArg::Trainer => Self::Trainer,
            ProfileArg::Horse => Self::Horse,
        }
    }
}

fn optional_date(value: Option<&String>) -> Result<Option<NaiveDate>> {
    value.map(|value| api::parse_date(value)).transpose()
}

fn split_types(value: Option<&String>) -> Option<Vec<String>> {
    value.map(|value| {
        value
            .split(',')
            .map(|part| part.trim().to_uppercase())
            .filter(|part| !part.is_empty())
            .collect()
    })
}

fn print_json(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new()?;

    if let Command::Mcp = cli.command {
        return hkjc::mcp::serve(client).await;
    }

    match cli.command {
        Command::Meetings { date, venue } => {
            let date = optional_date(date.as_ref())?;
            let data = client.meetings(date, venue.as_deref()).await?;
            let views = data.views_for(date, venue.as_deref());
            if cli.json {
                print_json(&views)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::meetings(&views));
            } else {
                println!("{}", hkjc::render::meetings(&views));
            }
        }
        Command::Card { race, date, venue } => {
            let date = optional_date(date.as_ref())?;
            let data = client.meetings(date, venue.as_deref()).await?;
            let meeting = data
                .pick(date, venue.as_deref())
                .ok_or_else(|| anyhow::anyhow!("no race meeting is currently published"))?;
            let found = meeting.race(race).ok_or_else(|| {
                anyhow::anyhow!(
                    "race {race} not found; the meeting has {} races",
                    meeting.races.len()
                )
            })?;
            let card = found.card(meeting);
            if cli.json {
                print_json(&card)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::card(&card));
            } else {
                println!("{}", hkjc::render::race_card(&card));
            }
        }
        Command::Odds {
            race,
            types,
            record,
            date,
            venue,
        } => {
            let date = optional_date(date.as_ref())?;
            let data = client.meetings(date, venue.as_deref()).await?;
            let meeting = data
                .pick(date, venue.as_deref())
                .ok_or_else(|| anyhow::anyhow!("no race meeting is currently published"))?;
            if meeting.race(race).is_none() {
                bail!(
                    "race {race} not found; the meeting has {} races",
                    meeting.races.len()
                );
            }
            let meeting_date = api::parse_date(&meeting.date)?;
            let odds = client
                .odds(
                    meeting_date,
                    &meeting.venue_code,
                    race,
                    split_types(types.as_ref()),
                )
                .await?;
            let view = odds.view(meeting_date, &meeting.venue_code, race);
            if record {
                let count = hkjc::history::record_odds(&view, cli.db.as_deref())?;
                eprintln!("recorded {count} odds entries for race {race}");
            }
            if cli.json {
                print_json(&view)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::odds(&view));
            } else {
                println!("{}", hkjc::render::odds(&view));
            }
        }
        Command::History {
            date,
            venue,
            race,
            pool,
            limit,
        } => {
            let date = match date {
                Some(value) => api::parse_date(&value)?,
                None => api::hong_kong_today(),
            };
            let view = hkjc::history::load(
                &date.to_string(),
                &venue,
                race,
                pool.as_deref(),
                limit,
                cli.db.as_deref(),
            )?;
            if cli.json {
                print_json(&view)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::history(&view));
            } else {
                println!("{}", hkjc::render::history(&view));
            }
        }
        Command::Results { date, venue, race } => {
            let date = match date {
                Some(value) => api::parse_date(&value)?,
                None => api::hong_kong_today(),
            };
            let result = match venue {
                Some(venue) => client.race_results(date, &venue, race).await?,
                None => match client.race_results(date, "ST", race).await {
                    Ok(result) => result,
                    Err(first) => match client.race_results(date, "HV", race).await {
                        Ok(result) => result,
                        Err(_) => return Err(first),
                    },
                },
            };
            if cli.json {
                print_json(&result)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::results(&result));
            } else {
                println!("{}", hkjc::render::race_result(&result));
            }
        }
        Command::Marksix { draws, from, to } => {
            let from = optional_date(from.as_ref())?;
            let to = optional_date(to.as_ref())?;
            let draws = client
                .mark_six(Some(draws), from, to)
                .await?
                .iter()
                .map(|draw| draw.view())
                .collect::<Vec<_>>();
            if cli.json {
                print_json(&draws)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::marksix(&draws));
            } else {
                println!("{}", hkjc::render::marksix(&draws));
            }
        }
        Command::MarksixLatest => {
            let draws = client.mark_six(Some(1), None, None).await?;
            let draw = draws
                .first()
                .ok_or_else(|| anyhow::anyhow!("no Mark Six draw is published"))?
                .view();
            if cli.json {
                print_json(&draw)?;
            } else {
                println!("{}", hkjc::render::marksix_draw(&draw));
            }
        }
        Command::MarksixStats { draws } => {
            let history = client.mark_six(Some(draws.max(1)), None, None).await?;
            let stats = api::mark_six_stats(&history);
            if cli.json {
                print_json(&stats)?;
            } else {
                println!("{}", hkjc::render::marksix_stats(&stats));
            }
        }
        Command::Fixtures {
            from,
            to,
            venue,
            all,
            ics,
            next,
        } => {
            let today = api::hong_kong_today();
            let (season_start, season_end) = api::season_bounds(today);
            let from = optional_date(from.as_ref())?;
            let to = optional_date(to.as_ref())?;
            let from = if all {
                from.unwrap_or(season_start)
            } else {
                from.unwrap_or(today)
            };
            let to = if next {
                season_end
            } else {
                to.unwrap_or_else(|| {
                    if all {
                        season_end
                    } else {
                        from.checked_add_days(Days::new(90)).unwrap_or(season_end)
                    }
                })
            };
            let fixtures = client.fixtures(from, to).await?;
            let mut views = api::fixture_views(&fixtures, Some(from), Some(to), venue.as_deref());
            if next {
                views.truncate(1);
                if views.is_empty() {
                    bail!("no further race meeting is published for this season");
                }
            }
            if ics {
                print!("{}", hkjc::render::fixtures_ics(&views));
            } else if cli.json {
                print_json(&views)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::fixtures(&views));
            } else {
                println!("{}", hkjc::render::fixtures(&views));
            }
        }
        Command::Standings {
            kind,
            season,
            venue,
        } => {
            let view = client
                .standings(kind.as_str(), season.as_deref(), venue.as_deref())
                .await?;
            if cli.json {
                print_json(&view)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::standings(&view));
            } else {
                println!("{}", hkjc::render::standings(&view));
            }
        }
        Command::Trials { date } => {
            let date = match date {
                Some(value) => api::parse_date(&value)?,
                None => api::hong_kong_today(),
            };
            let view = client.trials(date).await?;
            if cli.json {
                print_json(&view)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::trials(&view));
            } else {
                println!("{}", hkjc::render::trials(&view));
            }
        }
        Command::Sectionals { date, race } => {
            let date = match date {
                Some(value) => api::parse_date(&value)?,
                None => api::hong_kong_today(),
            };
            let view = client.sectionals(date, race).await?;
            if cli.json {
                print_json(&view)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::sectionals(&view));
            } else {
                println!("{}", hkjc::render::sectionals(&view));
            }
        }
        Command::Pace { date, race } => {
            let date = match date {
                Some(value) => api::parse_date(&value)?,
                None => api::hong_kong_today(),
            };
            let view = client.sectionals(date, race).await?;
            if cli.json {
                print_json(&view)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::pace(&view));
            } else {
                println!("{}", hkjc::render::pace(&view));
            }
        }
        Command::MarksixCheck { numbers, draw } => {
            let ticket = api::parse_ticket(&numbers)?;
            let draws = client
                .mark_six(Some(draw.map_or(1, |_| 30)), None, None)
                .await?;
            let selected = match draw {
                Some(selected) => {
                    draws
                        .iter()
                        .find(|item| item.no == selected)
                        .ok_or_else(|| {
                            anyhow::anyhow!("draw {selected} is not in the recent results")
                        })?
                }
                None => draws
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("no Mark Six draw is published"))?,
            };
            let view = api::check_mark_six(&ticket, &selected.view());
            if cli.json {
                print_json(&view)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::marksix_check(&view));
            } else {
                println!("{}", hkjc::render::marksix_check(&view));
            }
        }
        Command::Doctor => {
            let checks = client.health().await;
            if cli.json {
                print_json(&checks)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::health(&checks));
            } else {
                for check in &checks {
                    println!(
                        "{:<10} {:<5} {:>5} ms  {}",
                        check.name,
                        if check.ok { "ok" } else { "FAIL" },
                        check.millis,
                        if check.ok {
                            String::new()
                        } else {
                            check.detail.lines().next().unwrap_or("").to_string()
                        }
                    );
                }
                if let Some(failed) = checks.iter().find(|check| !check.ok) {
                    bail!("{} is unavailable: {}", failed.name, failed.detail);
                }
            }
        }
        Command::Football {
            from,
            to,
            tournament,
            limit,
        } => {
            let matches = client.football().await?;
            let views = api::football_views(
                &matches,
                optional_date(from.as_ref())?,
                optional_date(to.as_ref())?,
                tournament.as_deref(),
                limit,
            );
            if cli.json {
                print_json(&views)?;
            } else if cli.csv {
                print!("{}", hkjc::csv::football(&views));
            } else {
                println!("{}", hkjc::render::football(&views));
            }
        }
        Command::FootballMatch { id, types } => {
            let view = client
                .football_match(&id, split_types(types.as_ref()))
                .await?;
            if cli.json {
                print_json(&view)?;
            } else {
                println!("{}", hkjc::render::football_match(&view));
            }
        }
        Command::Profile { kind, id } => {
            let view = client.profile(kind.into(), &id).await?;
            if cli.json {
                print_json(&view)?;
            } else {
                println!("{}", hkjc::render::profile(&view));
            }
        }
        Command::Lookup { query } => {
            let hits = client.lookup(&query).await?;
            if cli.json {
                print_json(&hits)?;
            } else {
                println!("{}", hkjc::render::lookup(&hits));
            }
        }
        Command::Watch {
            date,
            venue,
            race,
            interval,
            count,
            webhook,
            telegram,
            record,
        } => {
            let options = hkjc::watch::WatchOptions {
                date: optional_date(date.as_ref())?,
                venue,
                race,
                interval,
                count,
                webhook,
                telegram,
                record,
                db: cli.db.clone(),
            };
            hkjc::watch::run(&client, options).await?;
        }
        Command::Completions { shell } => {
            let mut command = Cli::command();
            let name = command.get_name().to_string();
            clap_complete::generate(shell, &mut command, name, &mut std::io::stdout());
        }
        Command::Man => {
            let command = Cli::command();
            clap_mangen::Man::new(command).render(&mut std::io::stdout())?;
        }
        Command::Update { check, force } => update::run(check, force)?,
        Command::Mcp => unreachable!(),
    }
    Ok(())
}
