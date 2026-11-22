//! MCP for Hong Kong Jockey Club racing and Mark Six.
//!
//! The server speaks JSON-RPC over stdio, or streamable HTTP via
//! [`serve_http`]. The HTTP transport is stateless: every POST is a complete
//! request and the response is a JSON document.

use std::io::{BufRead, Write};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::api::{self, Client};

const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BODY_BYTES: usize = 1024 * 1024;

pub const PROTOCOL_VERSION: &str = "2025-06-18";
const SUPPORTED_VERSIONS: [&str; 2] = ["2025-06-18", "2024-11-05"];

pub async fn serve(client: Client) -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
    eprintln!("hkjc MCP server listening on stdio");
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(value) => dispatch(&client, &value).await,
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

/// Blocking entry point for binaries whose `main` is not async.
///
/// This builds the runtime at the top level, never inside an async context, so
/// it cannot hit "Cannot start a runtime from within a runtime".
pub fn serve_blocking(client: Client) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(serve(client))
}

/// Serve MCP's streamable HTTP transport.
///
/// `POST /mcp` accepts one JSON-RPC request and returns one JSON response.
/// Notifications get `202 Accepted`. `GET` is not supported because this
/// server has no server-initiated stream. A bearer token can be required with
/// `token`; a non-loopback bind refuses to start without one.
pub async fn serve_http(client: Client, address: &str, token: Option<String>) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("binding MCP HTTP server to {address}"))?;
    let local = listener.local_addr()?;
    if !local.ip().is_loopback() && token.is_none() {
        bail!("refusing to expose the MCP server on {local} without --token (or HKJC_MCP_TOKEN)");
    }
    eprintln!("hkjc MCP server listening on http://{local}/mcp");
    let client = Arc::new(client);
    let token = Arc::new(token);
    loop {
        let (stream, peer) = listener
            .accept()
            .await
            .with_context(|| "accepting an MCP HTTP connection")?;
        let client = Arc::clone(&client);
        let token = Arc::clone(&token);
        tokio::spawn(async move {
            if let Err(error) = handle_http(&client, stream, token.as_deref()).await {
                eprintln!("hkjc MCP HTTP {peer}: {error:#}");
            }
        });
    }
}

struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

async fn handle_http(client: &Client, mut stream: TcpStream, token: Option<&str>) -> Result<()> {
    let request = match read_http_request(&mut stream).await {
        Ok(request) => request,
        Err(error) => {
            respond(
                &mut stream,
                400,
                "Bad Request",
                "text/plain; charset=utf-8",
                error.to_string().as_bytes(),
                &[],
            )
            .await?;
            return Ok(());
        }
    };

    let origin = request.header("origin").unwrap_or_default().to_string();
    if !origin.is_empty() && !allowed_origin(&origin) {
        respond(
            &mut stream,
            403,
            "Forbidden",
            "text/plain; charset=utf-8",
            b"origin is not allowed\n",
            &[],
        )
        .await?;
        return Ok(());
    }
    let cors: Vec<(&str, &str)> = if origin.is_empty() {
        Vec::new()
    } else {
        vec![
            ("Access-Control-Allow-Origin", origin.as_str()),
            ("Vary", "Origin"),
            (
                "Access-Control-Allow-Headers",
                "authorization, content-type, mcp-protocol-version",
            ),
        ]
    };

    if request.method == "OPTIONS" {
        let mut headers = cors.clone();
        headers.push(("Access-Control-Allow-Methods", "POST, OPTIONS"));
        respond(
            &mut stream,
            204,
            "No Content",
            "text/plain; charset=utf-8",
            b"",
            &headers,
        )
        .await?;
        return Ok(());
    }
    if request.method != "POST" {
        let mut headers = cors.clone();
        headers.push(("Allow", "POST, OPTIONS"));
        respond(
            &mut stream,
            405,
            "Method Not Allowed",
            "text/plain; charset=utf-8",
            b"use POST /mcp for JSON-RPC\n",
            &headers,
        )
        .await?;
        return Ok(());
    }
    if request.path != "/mcp" && request.path != "/" {
        respond(
            &mut stream,
            404,
            "Not Found",
            "text/plain; charset=utf-8",
            b"MCP is served at /mcp\n",
            &cors,
        )
        .await?;
        return Ok(());
    }
    if let Some(token) = token
        && !bearer_matches(request.header("authorization"), token)
    {
        let mut headers = cors.clone();
        headers.push(("WWW-Authenticate", "Bearer"));
        respond(
            &mut stream,
            401,
            "Unauthorized",
            "text/plain; charset=utf-8",
            b"a valid bearer token is required\n",
            &headers,
        )
        .await?;
        return Ok(());
    }
    if !request
        .header("content-type")
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("application/json"))
    {
        respond(
            &mut stream,
            415,
            "Unsupported Media Type",
            "text/plain; charset=utf-8",
            b"content-type must be application/json\n",
            &cors,
        )
        .await?;
        return Ok(());
    }
    if let Some(version) = request.header("mcp-protocol-version")
        && !SUPPORTED_VERSIONS.contains(&version)
    {
        respond(
            &mut stream,
            400,
            "Bad Request",
            "text/plain; charset=utf-8",
            format!("unsupported MCP protocol version {version:?}\n").as_bytes(),
            &cors,
        )
        .await?;
        return Ok(());
    }

    let message: Value = match serde_json::from_slice(&request.body) {
        Ok(message) => message,
        Err(error) => {
            let body = error_response(None, -32700, &format!("parse error: {error}"));
            let body = serde_json::to_vec(&body)?;
            respond(&mut stream, 200, "OK", "application/json", &body, &cors).await?;
            return Ok(());
        }
    };
    if message.is_array() {
        let body = error_response(
            None,
            -32600,
            "invalid request: JSON-RPC batches are not supported",
        );
        let body = serde_json::to_vec(&body)?;
        respond(&mut stream, 200, "OK", "application/json", &body, &cors).await?;
        return Ok(());
    }

    match dispatch(client, &message).await {
        Some(response) => {
            let body = serde_json::to_vec(&response)?;
            respond(&mut stream, 200, "OK", "application/json", &body, &cors).await?;
        }
        None => {
            respond(
                &mut stream,
                202,
                "Accepted",
                "text/plain; charset=utf-8",
                b"",
                &cors,
            )
            .await?;
        }
    }
    Ok(())
}

async fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        if let Some(index) = find_bytes(&buffer, b"\r\n\r\n") {
            break index + 4;
        }
        if buffer.len() >= MAX_HEADER_BYTES {
            bail!("request headers are too large");
        }
        let read = stream
            .read(&mut chunk)
            .await
            .context("reading request headers")?;
        if read == 0 {
            bail!("connection closed before the request headers");
        }
        buffer.extend_from_slice(&chunk[..read]);
    };

    let head =
        std::str::from_utf8(&buffer[..header_end]).context("request headers are not UTF-8")?;
    let mut lines = head.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing request line"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing request method"))?
        .to_string();
    let target = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing request target"))?;
    let path = target.split('?').next().unwrap_or(target).to_string();
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("malformed request header"))?;
        headers.push((name.trim().to_string(), value.trim().to_string()));
    }

    let length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map(|(_, value)| value.parse::<usize>())
        .transpose()
        .context("invalid content-length")?
        .unwrap_or(0);
    if length > MAX_BODY_BYTES {
        bail!("request body is too large");
    }
    let mut body = buffer[header_end..].to_vec();
    if body.len() > MAX_BODY_BYTES {
        bail!("request body is too large");
    }
    while body.len() < length {
        let read = stream
            .read(&mut chunk)
            .await
            .context("reading request body")?;
        if read == 0 {
            bail!("connection closed before the request body was complete");
        }
        body.extend_from_slice(&chunk[..read]);
        if body.len() > MAX_BODY_BYTES {
            bail!("request body is too large");
        }
    }
    body.truncate(length);

    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn bearer_matches(header: Option<&str>, token: &str) -> bool {
    let Some((scheme, value)) = header.and_then(|value| value.split_once(' ')) else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("bearer") {
        return false;
    }
    let provided = value.trim().as_bytes();
    let expected = token.as_bytes();
    if provided.len() != expected.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in provided.iter().zip(expected) {
        difference |= left ^ right;
    }
    difference == 0
}

fn allowed_origin(origin: &str) -> bool {
    let Some((_, authority)) = origin.split_once("://") else {
        return false;
    };
    let authority = authority.split('/').next().unwrap_or(authority);
    let host = if let Some(rest) = authority.strip_prefix('[') {
        rest.split(']').next().unwrap_or_default()
    } else {
        authority.split(':').next().unwrap_or_default()
    };
    let host = host.to_ascii_lowercase();
    host == "localhost" || host == "127.0.0.1" || host == "::1"
}

async fn respond(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
    headers: &[(&str, &str)],
) -> Result<()> {
    let mut response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n",
        body.len()
    );
    for (name, value) in headers {
        response.push_str(name);
        response.push_str(": ");
        response.push_str(value);
        response.push_str("\r\n");
    }
    response.push_str("\r\n");
    stream
        .write_all(response.as_bytes())
        .await
        .context("writing response headers")?;
    stream
        .write_all(body)
        .await
        .context("writing response body")?;
    stream.flush().await.context("flushing response")?;
    Ok(())
}

async fn dispatch(client: &Client, request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = match request.get("method").and_then(Value::as_str) {
        Some(method) => method,
        None => {
            return Some(error_response(
                id,
                -32600,
                "invalid request: missing method",
            ));
        }
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
        },
        {
            "name": "hk_race_fixtures",
            "description": "Season fixture list of Hong Kong race meetings with dates and venues. Use for future meetings beyond the current live race card.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": {"type": "string", "description": "First date, YYYY-MM-DD. Default today, or the season start with all=true."},
                    "to": {"type": "string", "description": "Last date, YYYY-MM-DD. Default 90 days after from."},
                    "venue": {"type": "string", "description": "Venue code: ST, HV, CH or S1-S9."},
                    "all": {"type": "boolean", "description": "Return the whole published season."}
                }
            }
        },
        {
            "name": "hk_football_matches",
            "description": "HKJC football matches with status, score and HAD odds. Filter by date range or tournament.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": {"type": "string", "description": "First match date, YYYY-MM-DD."},
                    "to": {"type": "string", "description": "Last match date, YYYY-MM-DD."},
                    "tournament": {"type": "string", "description": "Tournament name or code substring."},
                    "limit": {"type": "integer", "minimum": 0, "description": "Maximum matches. Default 50; 0 means all."}
                }
            }
        },
        {
            "name": "hk_football_match",
            "description": "Full odds board for one HKJC football match: HAD, handicap, correct score, total goals and other pools.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {"type": "string", "description": "Match id from hk_football_matches."},
                    "types": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Pools to return. Default HAD, HHA, CRS, TTG."
                    }
                },
                "required": ["id"]
            }
        },
        {
            "name": "hk_race_profile",
            "description": "Official HKJC profile for a jockey, trainer or horse: age, background, season statistics and recent form.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "kind": {"type": "string", "enum": ["jockey", "trainer", "horse"], "description": "Profile kind."},
                    "id": {"type": "string", "description": "Jockey code, trainer code or horse id from hk_race_lookup."}
                },
                "required": ["kind", "id"]
            }
        },
        {
            "name": "hk_race_lookup",
            "description": "Find a jockey, trainer or horse in the current meeting by English or Chinese name, returning the id for hk_race_profile.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Name or part of a name."}
                },
                "required": ["query"]
            }
        },
        {
            "name": "hk_race_standings",
            "description": "Jockey or trainer championship standings for a season, with wins, placings, starts, win rate and stakes won, optionally limited to one racecourse.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "kind": {"type": "string", "enum": ["jockey", "trainer"], "description": "Championship table to return."},
                    "season": {"type": "string", "description": "Season code such as 25/26. Default: the current season."},
                    "venue": {"type": "string", "description": "Venue code: ST or HV. Default: both."}
                },
                "required": ["kind"]
            }
        },
        {
            "name": "hk_race_trials",
            "description": "Barrier trial results for one date: batches, distance, going, and each horse's rider, trainer, draw, running position and time.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "date": {"type": "string", "description": "Trial date, YYYY-MM-DD. Default today."}
                }
            }
        },
        {
            "name": "hk_race_sectionals",
            "description": "Sectional times for one race: race and sectional splits plus every runner's position, margin, sectional time and finish time. The per-runner positions also form the pace map.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "date": {"type": "string", "description": "Race date, YYYY-MM-DD. Default today."},
                    "race": {"type": "integer", "minimum": 1, "description": "Race number. Default 1."}
                }
            }
        },
        {
            "name": "hk_marksix_check",
            "description": "Check six Mark Six numbers against a draw, returning matched numbers, whether the extra number matches, the prize division and dividend.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "numbers": {
                        "type": "array",
                        "items": {"type": "integer", "minimum": 1, "maximum": 49},
                        "minItems": 6,
                        "maxItems": 6,
                        "description": "Six ticket numbers."
                    },
                    "draw": {"type": "integer", "minimum": 1, "description": "Draw number. Default: the latest draw."}
                },
                "required": ["numbers"]
            }
        },
        {
            "name": "hk_odds_history",
            "description": "Odds snapshots previously saved with `hkjc odds --record`: first and last odds per combination, movement and capture times. Empty until odds have been recorded locally.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "date": {"type": "string", "description": "Meeting date, YYYY-MM-DD. Default today."},
                    "venue": {"type": "string", "description": "Venue code: ST, HV or CH. Default ST."},
                    "race": {"type": "integer", "minimum": 1, "description": "Race number. Default 1."},
                    "pool": {"type": "string", "description": "Only this pool, e.g. WIN."},
                    "limit": {"type": "integer", "minimum": 0, "description": "Maximum combinations. Default 40; 0 means all."}
                }
            }
        },
        {
            "name": "hk_source_health",
            "description": "Probe each HKJC endpoint used by the skill and report availability, latency and failure detail.",
            "inputSchema": {"type": "object", "properties": {}}
        }
    ]})
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

fn arg_date(args: &Value, key: &str) -> Result<Option<chrono::NaiveDate>> {
    arg_str(args, key).map(api::parse_date).transpose()
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
                Ok(serde_json::to_value(
                    data.views_for(date, venue.as_deref()),
                )?)
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
                let draws = args
                    .get("draws")
                    .and_then(Value::as_i64)
                    .unwrap_or(100)
                    .max(1);
                let history = client.mark_six(Some(draws), None, None).await?;
                Ok(serde_json::to_value(api::mark_six_stats(&history))?)
            }
            "hk_race_fixtures" => {
                let all = args.get("all").and_then(Value::as_bool).unwrap_or(false);
                let today = api::hong_kong_today();
                let (season_start, season_end) = api::season_bounds(today);
                let from = arg_date(&args, "from")?;
                let to = arg_date(&args, "to")?;
                let from = if all {
                    from.unwrap_or(season_start)
                } else {
                    from.unwrap_or(today)
                };
                let to = to.unwrap_or_else(|| {
                    if all {
                        season_end
                    } else {
                        from.checked_add_days(chrono::Days::new(90))
                            .unwrap_or(season_end)
                    }
                });
                let fixtures = client.fixtures(from, to).await?;
                let views = api::fixture_views(
                    &fixtures,
                    Some(from),
                    Some(to),
                    arg_venue(&args).as_deref(),
                );
                Ok(serde_json::to_value(views)?)
            }
            "hk_football_matches" => {
                let matches = client.football().await?;
                let limit = args
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or(50);
                let views = api::football_views(
                    &matches,
                    arg_date(&args, "from")?,
                    arg_date(&args, "to")?,
                    arg_str(&args, "tournament"),
                    limit,
                );
                Ok(serde_json::to_value(views)?)
            }
            "hk_football_match" => {
                let id = arg_str(&args, "id").ok_or_else(|| anyhow::anyhow!("id is required"))?;
                let types = args.get("types").and_then(Value::as_array).map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|value| value.to_uppercase())
                        .collect::<Vec<_>>()
                });
                Ok(serde_json::to_value(
                    client.football_match(id, types).await?,
                )?)
            }
            "hk_race_profile" => {
                let kind = arg_str(&args, "kind")
                    .and_then(crate::model::ProfileKind::parse)
                    .ok_or_else(|| anyhow::anyhow!("kind must be jockey, trainer or horse"))?;
                let id = arg_str(&args, "id").ok_or_else(|| anyhow::anyhow!("id is required"))?;
                Ok(serde_json::to_value(client.profile(kind, id).await?)?)
            }
            "hk_race_lookup" => {
                let query =
                    arg_str(&args, "query").ok_or_else(|| anyhow::anyhow!("query is required"))?;
                Ok(serde_json::to_value(client.lookup(query).await?)?)
            }
            "hk_race_standings" => {
                let kind =
                    arg_str(&args, "kind").ok_or_else(|| anyhow::anyhow!("kind is required"))?;
                let view = client
                    .standings(kind, arg_str(&args, "season"), arg_venue(&args).as_deref())
                    .await?;
                Ok(serde_json::to_value(view)?)
            }
            "hk_race_trials" => {
                let date = arg_date(&args, "date")?.unwrap_or_else(api::hong_kong_today);
                Ok(serde_json::to_value(client.trials(date).await?)?)
            }
            "hk_race_sectionals" => {
                let date = arg_date(&args, "date")?.unwrap_or_else(api::hong_kong_today);
                let race = args.get("race").and_then(Value::as_i64).unwrap_or(1);
                Ok(serde_json::to_value(client.sectionals(date, race).await?)?)
            }
            "hk_marksix_check" => {
                let numbers = args
                    .get("numbers")
                    .and_then(Value::as_array)
                    .ok_or_else(|| anyhow::anyhow!("numbers must be an array of six integers"))?;
                let mut ticket = Vec::with_capacity(numbers.len());
                for number in numbers {
                    let value = number
                        .as_u64()
                        .filter(|value| (1..=49).contains(value))
                        .ok_or_else(|| anyhow::anyhow!("every Mark Six number must be 1-49"))?;
                    ticket.push(value as u8);
                }
                if ticket.len() != 6 {
                    bail!("numbers must contain exactly six Mark Six numbers");
                }
                let draw = args.get("draw").and_then(Value::as_i64);
                let draws = client
                    .mark_six(Some(draw.map_or(1, |_| 30)), None, None)
                    .await?;
                let selected =
                    match draw {
                        Some(selected) => draws
                            .iter()
                            .find(|item| item.no == selected)
                            .ok_or_else(|| {
                                anyhow::anyhow!("draw {selected} is not in the recent results")
                            })?,
                        None => draws
                            .first()
                            .ok_or_else(|| anyhow::anyhow!("no Mark Six draw is published"))?,
                    };
                Ok(serde_json::to_value(api::check_mark_six(
                    &ticket,
                    &selected.view(),
                ))?)
            }
            "hk_odds_history" => {
                let date = arg_date(&args, "date")?.unwrap_or_else(api::hong_kong_today);
                let venue = arg_venue(&args).unwrap_or_else(|| "ST".to_string());
                let race = args.get("race").and_then(Value::as_i64).unwrap_or(1);
                let limit = args
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or(40);
                let view = crate::history::load(
                    &date.to_string(),
                    &venue,
                    race,
                    arg_str(&args, "pool"),
                    limit,
                    None,
                )?;
                Ok(serde_json::to_value(view)?)
            }
            "hk_source_health" => Ok(serde_json::to_value(client.health().await)?),
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
        Err(error) => success(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": format!("{error:#}")
                }],
                "isError": true
            }),
        ),
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
