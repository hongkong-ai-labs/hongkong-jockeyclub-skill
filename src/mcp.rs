//! MCP over stdio for Hong Kong Jockey Club racing and Mark Six.

use std::io::{BufRead, Write};

use anyhow::{Result, bail};
use serde_json::{Value, json};

use crate::api::{self, Client};

pub const PROTOCOL_VERSION: &str = "2025-06-18";
const SUPPORTED_VERSIONS: [&str; 2] = ["2025-06-18", "2024-11-05"];

pub fn serve(client: Client) -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
    eprintln!("hkjc MCP server listening on stdio");
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(value) => {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(dispatch(&client, &value))
            }
            Err(error) => Some(error_response(
                None,
                -32700,
                &format!("parse error: {error}"),
            )),
        };
        if let Some(response) = response {
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }
    }
    Ok(())
}

async fn dispatch(client: &Client, request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = match request.get("method").and_then(Value::as_str) {
        Some(method) => method,
        None => return Some(error_response(id, -32600, "invalid request: missing method")),
    };
    if method.starts_with("notifications/") {
        return None;
    }
    match method {
        "initialize" => Some(success(id, initialize(request.get("params")))),
        "ping" => Some(success(id, json!({}))),
        "tools/list" => Some(success(id, tools())),
        "tools/call" => Some(call(client, id, request.get("params")).await),
        other => Some(error_response(
            id,
            -32601,
            &format!("method not found: {other}"),
        )),
    }
}

fn initialize(params: Option<&Value>) -> Value {
    let requested = params
        .and_then(|value| value.get("protocolVersion"))
        .and_then(Value::as_str);
    let version = requested
        .filter(|value| SUPPORTED_VERSIONS.contains(value))
        .unwrap_or(PROTOCOL_VERSION);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": {
            "name": "hongkong-jockeyclub-skill",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn tools() -> Value {
    json!({ "tools": [
        {
            "name": "hk_race_meetings",
            "description": "Current Hong Kong Jockey Club race meeting(s): date, venue, races, post times, class, distance and field size. Covers the meeting HKJC currently publishes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "date": {"type": "string", "description": "Meeting date, YYYY-MM-DD. Default: all active meetings."},
                    "venue": {"type": "string", "description": "Venue code: ST (Sha Tin), HV (Happy Valley), CH (Conghua) or S1-S9."}
                }
            }
        },
        {
            "name": "hk_race_card",
            "description": "Race card for one race: runners with horse names, jockeys, trainers, barrier draws, weights, ratings, recent form and win odds.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "race": {"type": "integer", "minimum": 1, "description": "Race number. Default 1."},
                    "date": {"type": "string", "description": "Meeting date, YYYY-MM-DD."},
                    "venue": {"type": "string", "description": "Venue code: ST, HV or CH."}
                }
            }
        },
        {
            "name": "hk_race_odds",
            "description": "Live tote odds for one race, by pool (WIN, PLA, QIN, QPL, FCT, TCE, TRI, FF, QTT, CWA, IWN and others).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "race": {"type": "integer", "minimum": 1, "description": "Race number. Default 1."},
                    "types": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Pools to return. Default WIN and PLA."
                    },
                    "date": {"type": "string", "description": "Meeting date, YYYY-MM-DD."},
                    "venue": {"type": "string", "description": "Venue code: ST, HV or CH."}
                }
            }
        },
        {
            "name": "hk_race_results",
            "description": "Historical race result: finishing order, jockey, trainer, weight, draw, lengths behind, finish time, starting odds, dividends and incidents.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "date": {"type": "string", "description": "Race date, YYYY-MM-DD. Default today."},
                    "venue": {"type": "string", "description": "Venue code: ST or HV. Default: try Sha Tin, then Happy Valley."},
                    "race": {"type": "integer", "minimum": 1, "description": "Race number. Default 1."}
                }
            }
        },
        {
            "name": "hk_marksix_results",
            "description": "Mark Six draw results: drawn numbers, extra number, jackpot, turnover and prize divisions. Filter by recent count or a date range.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "draws": {"type": "integer", "minimum": 1, "description": "Number of most recent draws. Default 10."},
                    "from": {"type": "string", "description": "First draw date, YYYY-MM-DD."},
                    "to": {"type": "string", "description": "Last draw date, YYYY-MM-DD."}
                }
            }
        },
        {
            "name": "hk_marksix_latest",
            "description": "The latest Mark Six draw with the full prize breakdown.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "hk_marksix_stats",
            "description": "Mark Six number frequency over recent draws: hot and cold numbers, odd/even split, average sum and last draw seen for each number.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "draws": {"type": "integer", "minimum": 1, "description": "Recent draws to analyse. Default 100."}
                }
            }
        }
    ]})
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

fn arg_date(args: &Value, key: &str) -> Result<Option<chrono::NaiveDate>> {
    arg_str(args, key)
        .map(api::parse_date)
        .transpose()
}

fn arg_venue(args: &Value) -> Option<String> {
    arg_str(args, "venue").map(|value| value.to_uppercase())
}

async fn call(client: &Client, id: Option<Value>, params: Option<&Value>) -> Value {
    let Some(params) = params else {
        return error_response(id, -32602, "missing params");
    };
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let outcome: Result<Value> = async {
        match name {
            "hk_race_meetings" => {
                let date = arg_date(&args, "date")?;
                let venue = arg_venue(&args);
                let data = client.meetings(date, venue.as_deref()).await?;
                Ok(serde_json::to_value(data.views())?)
            }
            "hk_race_card" => {
                let race = args.get("race").and_then(Value::as_i64).unwrap_or(1);
                let date = arg_date(&args, "date")?;
                let venue = arg_venue(&args);
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
                Ok(serde_json::to_value(found.card(meeting))?)
            }
            "hk_race_odds" => {
                let race = args.get("race").and_then(Value::as_i64).unwrap_or(1);
                let types = args.get("types").and_then(Value::as_array).map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|value| value.to_uppercase())
                        .collect::<Vec<_>>()
                });
                let date = arg_date(&args, "date")?;
                let venue = arg_venue(&args);
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
                    .odds(meeting_date, &meeting.venue_code, race, types)
                    .await?;
                Ok(serde_json::to_value(odds.view(
                    meeting_date,
                    &meeting.venue_code,
                    race,
                ))?)
            }
            "hk_race_results" => {
                let date = arg_date(&args, "date")?.unwrap_or_else(api::hong_kong_today);
                let race = args.get("race").and_then(Value::as_i64).unwrap_or(1);
                let result = match arg_venue(&args) {
                    Some(venue) => client.race_results(date, &venue, race).await?,
                    None => match client.race_results(date, "ST", race).await {
                        Ok(result) => result,
                        Err(first) => match client.race_results(date, "HV", race).await {
                            Ok(result) => result,
                            Err(_) => return Err(first),
                        },
                    },
                };
                Ok(serde_json::to_value(result)?)
            }
            "hk_marksix_results" => {
                let draws = args.get("draws").and_then(Value::as_i64).unwrap_or(10);
                let from = arg_date(&args, "from")?;
                let to = arg_date(&args, "to")?;
                let views = client
                    .mark_six(Some(draws), from, to)
                    .await?
                    .iter()
                    .map(|draw| draw.view())
                    .collect::<Vec<_>>();
                Ok(serde_json::to_value(views)?)
            }
            "hk_marksix_latest" => {
                let draws = client.mark_six(Some(1), None, None).await?;
                let draw = draws
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("no Mark Six draw is published"))?;
                Ok(serde_json::to_value(draw.view())?)
            }
            "hk_marksix_stats" => {
                let draws = args.get("draws").and_then(Value::as_i64).unwrap_or(100).max(1);
                let history = client.mark_six(Some(draws), None, None).await?;
                Ok(serde_json::to_value(api::mark_six_stats(&history))?)
            }
            other => bail!("unknown tool: {other}"),
        }
    }
    .await;
    match outcome {
        Ok(value) => success(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&value).unwrap_or_default()
                }],
                "structuredContent": value,
                "isError": false
            }),
        ),
        Err(error) => error_response(id, -32603, &format!("{error:#}")),
    }
}

fn success(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}
