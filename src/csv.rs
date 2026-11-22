//! CSV output for the table-shaped commands.
//!
//! One record per row, with a header. Nested structures (a meeting's races, a
//! pool's odds, a batch's horses) are flattened so the file stays rectangular.

use crate::model::*;

fn escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Render a header and rows as RFC 4180-style CSV.
pub fn render(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push_str(
        &headers
            .iter()
            .map(|value| escape(value))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push('\n');
    for row in rows {
        out.push_str(
            &row.iter()
                .map(|value| escape(value))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }
    out
}

pub fn meetings(meetings: &[MeetingView]) -> String {
    let rows = meetings
        .iter()
        .flat_map(|meeting| {
            meeting.races.iter().map(move |race| {
                vec![
                    meeting.date.clone(),
                    meeting.day_of_week.clone(),
                    meeting.venue.clone(),
                    meeting.venue_code.clone(),
                    meeting.status.clone(),
                    meeting.meeting_type.clone(),
                    race.no.to_string(),
                    race.post_time.clone(),
                    race.status.clone(),
                    race.race_name.clone(),
                    race.class.clone(),
                    race.distance.clone(),
                    race.track.clone(),
                    race.going.clone(),
                    race.field_size
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ]
            })
        })
        .collect::<Vec<_>>();
    render(
        &[
            "date",
            "day",
            "venue",
            "venue_code",
            "meeting_status",
            "meeting_type",
            "race",
            "post_time",
            "race_status",
            "race_name",
            "class",
            "distance",
            "track",
            "going",
            "field_size",
        ],
        &rows,
    )
}

pub fn card(card: &RaceCardView) -> String {
    let rows = card
        .runners
        .iter()
        .map(|runner| {
            vec![
                card.date.clone(),
                card.venue.clone(),
                card.race.no.to_string(),
                runner.no.to_string(),
                runner.horse_no.to_string(),
                runner.horse.clone(),
                runner.horse_zh.clone(),
                runner.jockey.clone(),
                runner.trainer.clone(),
                runner.draw.clone(),
                runner.weight.clone(),
                runner.rating.clone(),
                runner.last_6_runs.clone(),
                runner.gear.clone(),
                runner.status.clone(),
                runner.win_odds.clone(),
            ]
        })
        .collect::<Vec<_>>();
    render(
        &[
            "date",
            "venue",
            "race",
            "runner",
            "horse_no",
            "horse",
            "horse_zh",
            "jockey",
            "trainer",
            "draw",
            "weight",
            "rating",
            "last_6_runs",
            "gear",
            "status",
            "win_odds",
        ],
        &rows,
    )
}

pub fn odds(view: &OddsView) -> String {
    let rows = view
        .pools
        .iter()
        .flat_map(|pool| {
            pool.odds.iter().map(move |entry| {
                vec![
                    view.date.clone(),
                    view.venue.clone(),
                    view.race_no.to_string(),
                    pool.pool.clone(),
                    pool.status.clone(),
                    pool.last_update.clone(),
                    entry.combination.clone(),
                    entry.odds.clone(),
                    entry.hot_favourite.to_string(),
                ]
            })
        })
        .collect::<Vec<_>>();
    render(
        &[
            "date",
            "venue",
            "race",
            "pool",
            "status",
            "last_update",
            "combination",
            "odds",
            "hot_favourite",
        ],
        &rows,
    )
}

pub fn results(result: &RaceResultView) -> String {
    let rows = result
        .placings
        .iter()
        .map(|placing| {
            vec![
                result.date.clone(),
                result.venue.clone(),
                result.race_no.to_string(),
                result.race_id.clone(),
                result.race_name.clone(),
                placing.place.clone(),
                placing.horse_no.clone(),
                placing.horse.clone(),
                placing.jockey.clone(),
                placing.trainer.clone(),
                placing.actual_weight.clone(),
                placing.declared_weight.clone(),
                placing.draw.clone(),
                placing.lengths_behind.clone(),
                placing.running_position.clone(),
                placing.finish_time.clone(),
                placing.win_odds.clone(),
            ]
        })
        .collect::<Vec<_>>();
    render(
        &[
            "date",
            "venue",
            "race",
            "race_id",
            "race_name",
            "place",
            "horse_no",
            "horse",
            "jockey",
            "trainer",
            "actual_weight",
            "declared_weight",
            "draw",
            "lengths_behind",
            "running_position",
            "finish_time",
            "win_odds",
        ],
        &rows,
    )
}

pub fn marksix(draws: &[MarkSixDrawView]) -> String {
    let rows = draws
        .iter()
        .map(|draw| {
            let mut row = vec![
                draw.draw_date.clone(),
                draw.year.clone(),
                draw.draw_no.to_string(),
                draw.status.clone(),
                draw.snowball.clone(),
            ];
            row.extend(draw.numbers.iter().map(|number| number.to_string()));
            row.push(
                draw.extra
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            );
            row.push(draw.jackpot.clone());
            row.push(draw.turnover.clone());
            row.push(draw.estimated_first_prize.clone());
            row
        })
        .collect::<Vec<_>>();
    render(
        &[
            "date",
            "year",
            "draw_no",
            "status",
            "snowball",
            "n1",
            "n2",
            "n3",
            "n4",
            "n5",
            "n6",
            "extra",
            "jackpot",
            "turnover",
            "estimated_first_prize",
        ],
        &rows,
    )
}

pub fn fixtures(fixtures: &[FixtureView]) -> String {
    let rows = fixtures
        .iter()
        .map(|fixture| {
            vec![
                fixture.date.clone(),
                fixture.day_of_week.clone(),
                fixture.venue.clone(),
                fixture.venue_code.clone(),
            ]
        })
        .collect::<Vec<_>>();
    render(&["date", "day", "venue", "venue_code"], &rows)
}

pub fn standings(view: &StandingsView) -> String {
    let rows = view
        .rows
        .iter()
        .map(|row| {
            vec![
                row.rank.to_string(),
                row.name.clone(),
                row.name_zh.clone(),
                row.id.clone(),
                row.first.to_string(),
                row.second.to_string(),
                row.third.to_string(),
                row.fourth.to_string(),
                row.fifth.to_string(),
                row.starts.to_string(),
                row.stakes_won.clone(),
                format!("{:.1}", row.win_rate),
            ]
        })
        .collect::<Vec<_>>();
    render(
        &[
            "rank",
            "name",
            "name_zh",
            "id",
            "first",
            "second",
            "third",
            "fourth",
            "fifth",
            "starts",
            "stakes_won",
            "win_rate",
        ],
        &rows,
    )
}

pub fn trials(view: &TrialsView) -> String {
    let rows = view
        .batches
        .iter()
        .flat_map(|batch| {
            batch.horses.iter().map(move |horse| {
                vec![
                    view.date.clone(),
                    batch.batch.clone(),
                    batch.venue.clone(),
                    batch.distance.clone(),
                    batch.going.clone(),
                    horse.horse.clone(),
                    horse.horse_id.clone(),
                    horse.jockey.clone(),
                    horse.trainer.clone(),
                    horse.draw.clone(),
                    horse.gear.clone(),
                    horse.lengths_behind.clone(),
                    horse.running_position.clone(),
                    horse.time.clone(),
                    horse.result.clone(),
                    horse.comment.clone(),
                ]
            })
        })
        .collect::<Vec<_>>();
    render(
        &[
            "date",
            "batch",
            "venue",
            "distance",
            "going",
            "horse",
            "horse_id",
            "jockey",
            "trainer",
            "draw",
            "gear",
            "lbw",
            "running_position",
            "time",
            "result",
            "comment",
        ],
        &rows,
    )
}

pub fn sectionals(view: &SectionalsView) -> String {
    let sections = view
        .horses
        .iter()
        .map(|horse| horse.splits.len())
        .max()
        .unwrap_or(0);
    let mut headers = vec!["date", "race", "place", "horse_no", "horse"];
    let section_names = (1..=sections)
        .map(|index| format!("section_{index}"))
        .collect::<Vec<_>>();
    headers.extend(section_names.iter().map(String::as_str));
    headers.push("finish_time");
    let rows = view
        .horses
        .iter()
        .map(|horse| {
            let mut row = vec![
                view.date.clone(),
                view.race_no.to_string(),
                horse.place.clone(),
                horse.horse_no.clone(),
                horse.horse.clone(),
            ];
            for index in 0..sections {
                row.push(match horse.splits.get(index) {
                    Some(split) => {
                        let position = split
                            .position
                            .map(|value| value.to_string())
                            .unwrap_or_default();
                        format!("{position} {} {}", split.margin, split.time)
                            .trim()
                            .to_string()
                    }
                    None => String::new(),
                });
            }
            row.push(horse.finish_time.clone());
            row
        })
        .collect::<Vec<_>>();
    render(&headers, &rows)
}

pub fn football(matches: &[FootballMatchView]) -> String {
    let rows = matches
        .iter()
        .map(|item| {
            let had = item.odds.iter().find(|pool| pool.pool == "HAD");
            let line = had.and_then(|pool| pool.lines.first());
            let find = |label: &str| {
                line.and_then(|line| {
                    line.odds
                        .iter()
                        .find(|odd| odd.combination == label)
                        .map(|odd| odd.odds.clone())
                })
                .unwrap_or_default()
            };
            vec![
                item.date.clone(),
                item.kick_off.clone(),
                item.tournament.clone(),
                item.home.clone(),
                item.away.clone(),
                item.home_score.clone(),
                item.away_score.clone(),
                item.status.clone(),
                item.venue.clone(),
                find("H"),
                find("D"),
                find("A"),
            ]
        })
        .collect::<Vec<_>>();
    render(
        &[
            "date",
            "kick_off",
            "tournament",
            "home",
            "away",
            "home_score",
            "away_score",
            "status",
            "venue",
            "home_odds",
            "draw_odds",
            "away_odds",
        ],
        &rows,
    )
}

pub fn pace(view: &SectionalsView) -> String {
    let sections = view
        .horses
        .iter()
        .map(|horse| horse.splits.len())
        .max()
        .unwrap_or(0);
    let mut headers: Vec<String> = vec![
        "place".to_string(),
        "horse_no".to_string(),
        "horse".to_string(),
    ];
    headers.extend((1..=sections).map(|index| format!("s{index}")));
    headers.push("finish_time".to_string());
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    let rows = view
        .horses
        .iter()
        .map(|horse| {
            let mut row = vec![
                horse.place.clone(),
                horse.horse_no.clone(),
                horse.horse.clone(),
            ];
            for index in 0..sections {
                row.push(
                    horse
                        .splits
                        .get(index)
                        .and_then(|split| split.position)
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                );
            }
            row.push(horse.finish_time.clone());
            row
        })
        .collect::<Vec<_>>();
    render(&header_refs, &rows)
}

pub fn marksix_check(view: &MarkSixCheckView) -> String {
    let numbers = |values: &[u8]| {
        values
            .iter()
            .map(|number| format!("{number:02}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    render(
        &[
            "draw_date",
            "draw_no",
            "ticket",
            "drawn",
            "extra",
            "matches",
            "extra_matched",
            "division",
            "dividend",
        ],
        &[vec![
            view.draw_date.clone(),
            view.draw_no.to_string(),
            numbers(&view.ticket),
            numbers(&view.drawn),
            view.extra.map(|v| v.to_string()).unwrap_or_default(),
            numbers(&view.matches),
            view.extra_matched.to_string(),
            view.division.map(|v| v.to_string()).unwrap_or_default(),
            view.dividend.clone(),
        ]],
    )
}

pub fn health(checks: &[HealthCheck]) -> String {
    let rows = checks
        .iter()
        .map(|check| {
            vec![
                check.name.clone(),
                check.url.clone(),
                check.ok.to_string(),
                check.detail.clone(),
                check.millis.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    render(&["name", "url", "ok", "detail", "millis"], &rows)
}

pub fn history(view: &HistoryView) -> String {
    let rows = view
        .entries
        .iter()
        .flat_map(|entry| {
            entry.points.iter().enumerate().map(move |(index, point)| {
                vec![
                    view.meeting_date.clone(),
                    view.venue.clone(),
                    view.race_no.to_string(),
                    entry.pool.clone(),
                    entry.combination.clone(),
                    point.captured_at.clone(),
                    point.odds.clone(),
                    index.to_string(),
                ]
            })
        })
        .collect::<Vec<_>>();
    render(
        &[
            "meeting_date",
            "venue",
            "race",
            "pool",
            "combination",
            "captured_at",
            "odds",
            "point",
        ],
        &rows,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pace_lists_each_runner_and_section() {
        let view = SectionalsView {
            date: "2026-11-22".to_string(),
            race_no: 1,
            race_times: vec!["23.88".to_string(), "46.25".to_string()],
            sectional_times: vec!["23.88".to_string(), "22.37".to_string()],
            horses: vec![SectionalHorseView {
                place: "1".to_string(),
                horse_no: "6".to_string(),
                horse: "FREE PONY (K229)".to_string(),
                splits: vec![
                    SectionalSplitView {
                        position: Some(8),
                        margin: "3".to_string(),
                        time: "24.36".to_string(),
                        fractions: Vec::new(),
                    },
                    SectionalSplitView {
                        position: Some(1),
                        margin: "N".to_string(),
                        time: "23.23".to_string(),
                        fractions: Vec::new(),
                    },
                ],
                finish_time: "1:09.88".to_string(),
            }],
        };
        let csv = pace(&view);
        let mut lines = csv.lines();
        assert_eq!(lines.next(), Some("place,horse_no,horse,s1,s2,finish_time"));
        assert_eq!(lines.next(), Some("1,6,FREE PONY (K229),8,1,1:09.88"));
    }
}
