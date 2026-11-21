//! Response models for the HKJC GraphQL API and the racing results pages.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// HKJC mixes JSON numbers and numeric strings in the same field.
fn de_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(match value {
        Value::Number(number) => number.as_i64().unwrap_or(0),
        Value::String(text) => text.trim().parse().unwrap_or(0),
        _ => 0,
    })
}

/// GraphQL response envelope.
#[derive(Debug, Deserialize)]
pub struct Envelope<T> {
    pub data: Option<T>,
    #[serde(default)]
    pub errors: Vec<GqlError>,
}

#[derive(Debug, Deserialize)]
pub struct GqlError {
    pub message: String,
}

pub fn scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

pub fn text(value: &Option<Value>) -> String {
    value.as_ref().map(scalar).unwrap_or_default()
}

/// Race meetings query response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingsData {
    #[serde(default)]
    pub race_meetings: Vec<Meeting>,
    #[serde(default)]
    pub active_meetings: Vec<ActiveMeeting>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveMeeting {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub venue_code: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub races: Vec<ActiveRace>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveRace {
    #[serde(default, deserialize_with = "de_i64")]
    pub no: i64,
    #[serde(default)]
    pub post_time: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub wagering_field_size: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meeting {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub venue_code: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub total_number_of_race: Option<i64>,
    #[serde(default)]
    pub current_number_of_race: Option<i64>,
    #[serde(default)]
    pub date_of_week: Option<String>,
    #[serde(default)]
    pub meeting_type: Option<String>,
    #[serde(default)]
    pub country: Vec<Country>,
    #[serde(default)]
    pub races: Vec<Race>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Country {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub nameen: String,
    #[serde(default)]
    pub namech: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Race {
    #[serde(default)]
    pub id: String,
    #[serde(default, deserialize_with = "de_i64")]
    pub no: i64,
    #[serde(default)]
    pub status: String,
    #[serde(default, rename = "raceName_en")]
    pub race_name_en: String,
    #[serde(default, rename = "raceName_ch")]
    pub race_name_ch: String,
    #[serde(default)]
    pub post_time: Option<String>,
    #[serde(default)]
    pub distance: Option<Value>,
    #[serde(default)]
    pub wagering_field_size: Option<i64>,
    #[serde(default, rename = "go_en")]
    pub going_en: String,
    #[serde(default)]
    pub cla_code: String,
    #[serde(default, rename = "raceClass_en")]
    pub race_class_en: String,
    #[serde(default, rename = "raceClass_ch")]
    pub race_class_ch: String,
    #[serde(default, rename = "country_en")]
    pub country_en: String,
    #[serde(default, rename = "ratingType")]
    pub rating_type: String,
    #[serde(default, rename = "raceTrack")]
    pub race_track: Option<Descriptions>,
    #[serde(default, rename = "raceCourse")]
    pub race_course: Option<Course>,
    #[serde(default)]
    pub runners: Vec<Runner>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptions {
    #[serde(default, rename = "description_en")]
    pub description_en: String,
    #[serde(default, rename = "description_ch")]
    pub description_ch: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Course {
    #[serde(default, rename = "displayCode")]
    pub display_code: String,
    #[serde(default, rename = "description_en")]
    pub description_en: String,
    #[serde(default, rename = "description_ch")]
    pub description_ch: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Runner {
    #[serde(default)]
    pub id: String,
    #[serde(default, deserialize_with = "de_i64")]
    pub no: i64,
    #[serde(default)]
    pub status: String,
    #[serde(default, rename = "name_en")]
    pub name_en: String,
    #[serde(default, rename = "name_ch")]
    pub name_ch: String,
    #[serde(default)]
    pub horse: Option<CodeName>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub barrier_draw_number: Option<Value>,
    #[serde(default)]
    pub handicap_weight: Option<Value>,
    #[serde(default)]
    pub current_weight: Option<Value>,
    #[serde(default)]
    pub current_rating: Option<Value>,
    #[serde(default)]
    pub international_rating: Option<Value>,
    #[serde(default)]
    pub gear_info: Option<String>,
    #[serde(default)]
    pub allowance: Option<Value>,
    #[serde(default)]
    pub last6run: Option<String>,
    #[serde(default)]
    pub saddle_cloth_no: Option<Value>,
    #[serde(default)]
    pub final_position: Option<Value>,
    #[serde(default)]
    pub win_odds: Option<Value>,
    #[serde(default)]
    pub jockey: Option<Person>,
    #[serde(default)]
    pub trainer: Option<Person>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeName {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub code: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    #[serde(default)]
    pub code: String,
    #[serde(default, rename = "name_en")]
    pub name_en: String,
    #[serde(default, rename = "name_ch")]
    pub name_ch: String,
}

/// Result meetings query response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultsData {
    #[serde(default)]
    pub race_meetings: Vec<ResultMeeting>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultMeeting {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub res_pools: Vec<ResultPool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultPool {
    #[serde(default)]
    pub leg: Option<Leg>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub odds_type: String,
    #[serde(default, rename = "name_en")]
    pub name_en: Option<String>,
    #[serde(default, rename = "name_ch")]
    pub name_ch: Option<String>,
    #[serde(default)]
    pub last_update_time: Option<String>,
    #[serde(default)]
    pub dividends: Vec<Dividend>,
    #[serde(default)]
    pub c_win_selections: Vec<CWinSelection>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dividend {
    #[serde(default)]
    pub win_comb: String,
    #[serde(default, rename = "type")]
    pub kind: Option<Value>,
    #[serde(default)]
    pub div: Option<Value>,
    #[serde(default)]
    pub seq: Option<Value>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub guarantee: Option<Value>,
    #[serde(default)]
    pub partial: Option<Value>,
    #[serde(default)]
    pub partial_unit: Option<Value>,
}

/// Odds query response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OddsData {
    #[serde(default)]
    pub race_meetings: Vec<OddsMeeting>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OddsMeeting {
    #[serde(default)]
    pub pm_pools: Vec<PmPool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PmPool {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub sell_status: String,
    #[serde(default)]
    pub odds_type: String,
    #[serde(default)]
    pub last_update_time: Option<String>,
    #[serde(default)]
    pub guarantee: Option<Value>,
    #[serde(default)]
    pub min_ticket_cost: Option<Value>,
    #[serde(default, rename = "name_en")]
    pub name_en: Option<String>,
    #[serde(default, rename = "name_ch")]
    pub name_ch: Option<String>,
    #[serde(default)]
    pub leg: Option<Leg>,
    #[serde(default)]
    pub c_win_selections: Vec<CWinSelection>,
    #[serde(default)]
    pub odds_nodes: Vec<OddsNode>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Leg {
    #[serde(default)]
    pub number: Option<Value>,
    #[serde(default)]
    pub races: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CWinSelection {
    #[serde(default)]
    pub composite: String,
    #[serde(default, rename = "name_ch")]
    pub name_ch: String,
    #[serde(default, rename = "name_en")]
    pub name_en: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OddsNode {
    #[serde(default)]
    pub comb_string: String,
    #[serde(default)]
    pub odds_value: Option<Value>,
    #[serde(default)]
    pub hot_favourite: Option<bool>,
    #[serde(default)]
    pub odds_drop_value: Option<Value>,
    #[serde(default)]
    pub banker_odds: Vec<BankerOdds>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankerOdds {
    #[serde(default)]
    pub comb_string: String,
    #[serde(default)]
    pub odds_value: Option<Value>,
}

/// Mark Six query response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawsData {
    #[serde(default)]
    pub lottery_draws: Vec<Draw>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draw {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub year: String,
    #[serde(default, deserialize_with = "de_i64")]
    pub no: i64,
    #[serde(default)]
    pub open_date: Option<String>,
    #[serde(default)]
    pub close_date: Option<String>,
    #[serde(default)]
    pub draw_date: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub snowball_code: String,
    #[serde(default, rename = "snowballName_en")]
    pub snowball_name_en: String,
    #[serde(default, rename = "snowballName_ch")]
    pub snowball_name_ch: String,
    #[serde(default)]
    pub lottery_pool: Option<LotteryPool>,
    #[serde(default)]
    pub draw_result: Option<DrawResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LotteryPool {
    #[serde(default)]
    pub sell: bool,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub total_investment: Option<Value>,
    #[serde(default)]
    pub jackpot: Option<Value>,
    #[serde(default)]
    pub unit_bet: Option<Value>,
    #[serde(default)]
    pub estimated_prize: Option<Value>,
    #[serde(default)]
    pub derived_first_prize_div: Option<Value>,
    #[serde(default)]
    pub lottery_prizes: Vec<Prize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prize {
    #[serde(default, rename = "type", deserialize_with = "de_i64")]
    pub kind: i64,
    #[serde(default)]
    pub winning_unit: Option<Value>,
    #[serde(default)]
    pub dividend: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawResult {
    #[serde(default)]
    pub drawn_no: Vec<i64>,
    #[serde(default)]
    pub x_drawn_no: Option<i64>,
}

// ---------------------------------------------------------------------------
// Views returned by the CLI and MCP tools
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct MeetingView {
    pub id: String,
    pub date: String,
    pub venue_code: String,
    pub venue: String,
    pub status: String,
    pub day_of_week: String,
    pub meeting_type: String,
    pub country: String,
    pub total_races: usize,
    pub current_race: Option<i64>,
    pub races: Vec<RaceView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RaceView {
    pub no: i64,
    pub post_time: String,
    pub status: String,
    pub distance: String,
    pub class: String,
    pub race_name: String,
    pub going: String,
    pub track: String,
    pub course: String,
    pub field_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunnerView {
    pub no: i64,
    pub horse_no: i64,
    pub horse: String,
    pub horse_zh: String,
    pub horse_id: String,
    pub jockey: String,
    pub jockey_code: String,
    pub trainer: String,
    pub trainer_code: String,
    pub draw: String,
    pub weight: String,
    pub rating: String,
    pub last_6_runs: String,
    pub gear: String,
    pub status: String,
    pub win_odds: String,
    pub final_position: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RaceCardView {
    pub meeting_id: String,
    pub date: String,
    pub venue_code: String,
    pub venue: String,
    pub race: RaceView,
    pub runners: Vec<RunnerView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OddsPoolView {
    pub pool: String,
    pub name: String,
    pub status: String,
    pub sell_status: String,
    pub last_update: String,
    pub legs: Vec<i64>,
    pub odds: Vec<OddsEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OddsEntry {
    pub combination: String,
    pub odds: String,
    pub hot_favourite: bool,
    pub bankers: Vec<OddsEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OddsView {
    pub date: String,
    pub venue_code: String,
    pub venue: String,
    pub race_no: i64,
    pub pools: Vec<OddsPoolView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarkSixDrawView {
    pub draw_id: String,
    pub year: String,
    pub draw_no: i64,
    pub draw_date: String,
    pub status: String,
    pub snowball: String,
    pub numbers: Vec<u8>,
    pub extra: Option<u8>,
    pub jackpot: String,
    pub turnover: String,
    pub unit_bet: String,
    pub estimated_first_prize: String,
    pub prizes: Vec<MarkSixPrizeView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarkSixPrizeView {
    pub division: i64,
    pub winning_units: String,
    pub dividend: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NumberStat {
    pub number: u8,
    pub count: u64,
    pub last_seen_draw: Option<String>,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarkSixStatsView {
    pub draws_analysed: usize,
    pub from_draw: Option<String>,
    pub to_draw: Option<String>,
    pub hot: Vec<NumberStat>,
    pub cold: Vec<NumberStat>,
    pub numbers: Vec<NumberStat>,
    pub odd_even: OddEven,
    pub average_sum: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OddEven {
    pub odd: u64,
    pub even: u64,
}

/// A parsed historical race result from racing.hkjc.com.
#[derive(Debug, Clone, Serialize)]
pub struct RaceResultView {
    pub date: String,
    pub venue: String,
    pub race_no: i64,
    pub race_id: String,
    pub race_name: String,
    pub class: String,
    pub distance: String,
    pub going: String,
    pub course: String,
    pub prize: String,
    pub finish_time: String,
    pub placings: Vec<PlacingView>,
    pub dividends: Vec<DividendView>,
    pub incidents: Vec<IncidentView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlacingView {
    pub place: String,
    pub horse_no: String,
    pub horse: String,
    pub jockey: String,
    pub trainer: String,
    pub actual_weight: String,
    pub declared_weight: String,
    pub draw: String,
    pub lengths_behind: String,
    pub running_position: String,
    pub finish_time: String,
    pub win_odds: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DividendView {
    pub pool: String,
    pub combination: String,
    pub dividend: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IncidentView {
    pub place: String,
    pub horse_no: String,
    pub horse: String,
    pub incident: String,
}

// ---------------------------------------------------------------------------
// Season fixtures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fixture {
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub venue_code: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FixtureView {
    pub date: String,
    pub day_of_week: String,
    pub venue_code: String,
    pub venue: String,
}

// ---------------------------------------------------------------------------
// Football
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FootballData {
    #[serde(default)]
    pub matches: Vec<FootballMatch>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FootballMatch {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub front_end_id: Option<String>,
    #[serde(default)]
    pub match_date: String,
    #[serde(default)]
    pub kick_off_time: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub sequence: Option<Value>,
    #[serde(default)]
    pub home_team: Option<Team>,
    #[serde(default)]
    pub away_team: Option<Team>,
    #[serde(default)]
    pub tournament: Option<Tournament>,
    #[serde(default)]
    pub venue: Option<Venue>,
    #[serde(default)]
    pub tv_channels: Vec<Venue>,
    #[serde(default)]
    pub running_result: Option<Score>,
    #[serde(default)]
    pub running_result_extra: Option<Score>,
    #[serde(default)]
    pub fo_pools: Vec<FoPool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    #[serde(default)]
    pub id: String,
    #[serde(default, rename = "name_en")]
    pub name_en: String,
    #[serde(default, rename = "name_ch")]
    pub name_ch: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tournament {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub code: String,
    #[serde(default, rename = "name_en")]
    pub name_en: String,
    #[serde(default, rename = "name_ch")]
    pub name_ch: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Venue {
    #[serde(default)]
    pub code: String,
    #[serde(default, rename = "name_en")]
    pub name_en: String,
    #[serde(default, rename = "name_ch")]
    pub name_ch: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Score {
    #[serde(default)]
    pub home_score: Option<Value>,
    #[serde(default)]
    pub away_score: Option<Value>,
    #[serde(default)]
    pub corner: Option<Value>,
    #[serde(default)]
    pub home_corner: Option<Value>,
    #[serde(default)]
    pub away_corner: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoPool {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub odds_type: String,
    #[serde(default)]
    pub inst_no: Option<Value>,
    #[serde(default)]
    pub inplay: Option<Value>,
    #[serde(default, rename = "name_en")]
    pub name_en: String,
    #[serde(default, rename = "name_ch")]
    pub name_ch: String,
    #[serde(default)]
    pub lines: Vec<FoLine>,
    #[serde(default)]
    pub closed_lines: Vec<FoLine>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoLine {
    #[serde(default)]
    pub line_id: Option<Value>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub condition: Option<Value>,
    #[serde(default)]
    pub main: Option<Value>,
    #[serde(default)]
    pub combinations: Vec<FoCombination>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoCombination {
    #[serde(default)]
    pub comb_id: Option<Value>,
    #[serde(default, rename = "str")]
    pub label: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub current_odds: Option<Value>,
    #[serde(default)]
    pub selections: Vec<FoSelection>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoSelection {
    #[serde(default)]
    pub sel_id: Option<Value>,
    #[serde(default, rename = "str")]
    pub label: String,
    #[serde(default, rename = "name_en")]
    pub name_en: String,
    #[serde(default, rename = "name_ch")]
    pub name_ch: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FootballMatchView {
    pub id: String,
    pub date: String,
    pub kick_off: String,
    pub status: String,
    pub tournament: String,
    pub home: String,
    pub away: String,
    pub home_score: String,
    pub away_score: String,
    pub venue: String,
    pub odds: Vec<FootballPoolView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FootballPoolView {
    pub pool: String,
    pub name: String,
    pub status: String,
    pub lines: Vec<FootballLineView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FootballLineView {
    pub condition: String,
    pub odds: Vec<FootballOddView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FootballOddView {
    pub combination: String,
    pub label: String,
    pub odds: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// Jockey / trainer / horse profiles
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    Jockey,
    Trainer,
    Horse,
}

impl ProfileKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "jockey" | "jockeys" => Some(Self::Jockey),
            "trainer" | "trainers" => Some(Self::Trainer),
            "horse" | "horses" => Some(Self::Horse),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Jockey => "jockey",
            Self::Trainer => "trainer",
            Self::Horse => "horse",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileStat {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HorseRunView {
    pub season: String,
    pub index: String,
    pub place: String,
    pub date: String,
    pub track: String,
    pub distance: String,
    pub going: String,
    pub class: String,
    pub draw: String,
    pub rating: String,
    pub trainer: String,
    pub jockey: String,
    pub lengths_behind: String,
    pub win_odds: String,
    pub weight: String,
    pub finish_time: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileView {
    pub kind: String,
    pub id: String,
    pub name: String,
    pub age: String,
    pub background: String,
    pub season: String,
    pub stats: Vec<ProfileStat>,
    pub form: Vec<HorseRunView>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LookupHit {
    pub kind: String,
    pub name: String,
    pub name_zh: String,
    pub id: String,
    pub meeting: String,
    pub race_no: i64,
    pub horse: String,
}
