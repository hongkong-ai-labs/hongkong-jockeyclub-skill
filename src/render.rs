//! Human-readable tables and detail views.

use crate::model::*;

fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers
        .iter()
        .map(|header| header.chars().count())
        .collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if index < widths.len() {
                widths[index] = widths[index].max(cell.chars().count());
            }
        }
    }
    let mut out = String::new();
    out.push_str(
        headers
            .iter()
            .enumerate()
            .map(|(index, header)| format!("{header:<width$}", width = widths[index]))
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end(),
    );
    out.push('\n');
    out.push_str(
        &widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<_>>()
            .join("  "),
    );
    out.push('\n');
    for row in rows {
        out.push_str(
            row.iter()
                .enumerate()
                .map(|(index, cell)| {
                    let width = widths.get(index).copied().unwrap_or(0);
                    let cell = truncate(cell, width);
                    format!("{cell:<width$}")
                })
                .collect::<Vec<_>>()
                .join("  ")
                .trim_end(),
        );
        out.push('\n');
    }
    out
}

fn truncate(value: &str, limit: usize) -> String {
    if limit == 0 || value.chars().count() <= limit {
        return value.to_string();
    }
    let mut out: String = value.chars().take(limit.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn group(mut value: i64) -> String {
    let negative = value < 0;
    value = value.abs();
    let digits = value.to_string();
    let mut out = String::new();
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(character);
    }
    if negative { format!("-{out}") } else { out }
}

fn money(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "-".to_string();
    }
    let cleaned: String = trimmed
        .chars()
        .filter(|character| *character != ',' && *character != '$')
        .collect();
    let Ok(amount) = cleaned.trim().parse::<f64>() else {
        return trimmed.to_string();
    };
    let cents = (amount * 100.0).round() as i64;
    let whole = cents / 100;
    let fraction = (cents % 100).abs();
    if fraction == 0 {
        format!("HK${}", group(whole))
    } else {
        format!("HK${}.{fraction:02}", group(whole))
    }
}

pub fn meetings(meetings: &[MeetingView]) -> String {
    if meetings.is_empty() {
        return "No race meeting is currently published.".to_string();
    }
    let mut out = String::new();
    for (index, meeting) in meetings.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&format!(
            "{} — {} ({}){}\n",
            meeting.date,
            meeting.venue,
            meeting.venue_code,
            if meeting.day_of_week.is_empty() {
                String::new()
            } else {
                format!(", {}", meeting.day_of_week)
            }
        ));
        out.push_str(&format!(
            "{} races  ·  meeting {}  ·  status {}\n",
            meeting.total_races, meeting.id, meeting.status
        ));
        let rows: Vec<Vec<String>> = meeting
            .races
            .iter()
            .map(|race| {
                vec![
                    race.no.to_string(),
                    race.post_time.clone(),
                    race.race_name.clone(),
                    race.class.clone(),
                    race.distance.clone(),
                    race.going.clone(),
                    race.field_size
                        .map(|size| size.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    race.status.clone(),
                ]
            })
            .collect();
        out.push_str(
            table(
                &[
                    "R", "TIME", "RACE", "CLASS", "DIST", "GOING", "RUNNERS", "STATUS",
                ],
                &rows,
            )
            .trim_end(),
        );
        out.push('\n');
    }
    out.trim_end().to_string()
}

pub fn race_card(card: &RaceCardView) -> String {
    let race = &card.race;
    let mut out = String::new();
    out.push_str(&format!(
        "{} {} — Race {} · {} {}{}\n",
        card.date,
        card.venue,
        race.no,
        race.post_time,
        race.race_name,
        if race.class.is_empty() {
            String::new()
        } else {
            format!(" · {}", race.class)
        }
    ));
    let distance = if race.distance.is_empty() {
        String::new()
    } else {
        format!("{}m", race.distance)
    };
    let declared = card
        .runners
        .iter()
        .filter(|runner| runner.status.eq_ignore_ascii_case("declared"))
        .count();
    let standby = card.runners.len().saturating_sub(declared);
    out.push_str(&format!(
        "{} · {} · {} · {} runners{}\n\n",
        distance,
        race.going,
        race.course,
        declared,
        if standby > 0 {
            format!(" (+{standby} standby)")
        } else {
            String::new()
        }
    ));
    let rows: Vec<Vec<String>> = card
        .runners
        .iter()
        .map(|runner| {
            vec![
                if runner.horse_no == 0 {
                    "-".to_string()
                } else {
                    runner.horse_no.to_string()
                },
                runner.horse.clone(),
                runner.jockey.clone(),
                runner.trainer.clone(),
                runner.draw.clone(),
                runner.weight.clone(),
                runner.rating.clone(),
                runner.last_6_runs.clone(),
                runner.win_odds.clone(),
                runner.status.clone(),
            ]
        })
        .collect();
    out.push_str(
        table(
            &[
                "NO", "HORSE", "JOCKEY", "TRAINER", "DRAW", "WT", "RTG", "LAST 6", "WIN", "STATUS",
            ],
            &rows,
        )
        .trim_end(),
    );
    out
}

pub fn odds(view: &OddsView) -> String {
    if view.pools.is_empty() {
        return format!(
            "No odds are published for {} {} race {}.",
            view.date, view.venue, view.race_no
        );
    }
    let mut out = String::new();
    out.push_str(&format!(
        "{} {} — Race {} odds\n",
        view.date, view.venue, view.race_no
    ));
    for pool in &view.pools {
        out.push_str(&format!(
            "\n{} ({}) · {} · updated {}\n",
            pool.name,
            pool.pool,
            pool.sell_status,
            if pool.last_update.is_empty() {
                "-"
            } else {
                &pool.last_update
            }
        ));
        if pool.odds.is_empty() {
            out.push_str("  no odds yet\n");
            continue;
        }
        let rows: Vec<Vec<String>> = pool
            .odds
            .iter()
            .map(|entry| {
                vec![
                    entry.combination.clone(),
                    entry.odds.clone(),
                    if entry.hot_favourite {
                        "★".to_string()
                    } else {
                        String::new()
                    },
                ]
            })
            .collect();
        out.push_str(&table(&["COMBINATION", "ODDS", ""], &rows));
    }
    out.trim_end().to_string()
}

pub fn marksix(draws: &[MarkSixDrawView]) -> String {
    if draws.is_empty() {
        return "No Mark Six draw found.".to_string();
    }
    let rows: Vec<Vec<String>> = draws
        .iter()
        .map(|draw| {
            vec![
                draw.draw_no.to_string(),
                draw.draw_date.clone(),
                draw.numbers
                    .iter()
                    .map(|number| format!("{number:02}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                draw.extra
                    .map(|number| format!("{number:02}"))
                    .unwrap_or_else(|| "-".to_string()),
                money(&draw.jackpot),
                money(&draw.turnover),
                draw.snowball.clone(),
            ]
        })
        .collect();
    table(
        &[
            "DRAW", "DATE", "NUMBERS", "+", "JACKPOT", "TURNOVER", "SNOWBALL",
        ],
        &rows,
    )
    .trim_end()
    .to_string()
}

pub fn marksix_draw(draw: &MarkSixDrawView) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Mark Six draw {} — {}\n\n",
        draw.draw_no, draw.draw_date
    ));
    out.push_str(&format!(
        "Numbers: {}\n",
        draw.numbers
            .iter()
            .map(|number| format!("{number:02}"))
            .collect::<Vec<_>>()
            .join("  ")
    ));
    if let Some(extra) = draw.extra {
        out.push_str(&format!("Extra:   {extra:02}\n"));
    }
    if !draw.snowball.is_empty() {
        out.push_str(&format!("Snowball: {}\n", draw.snowball));
    }
    out.push_str(&format!("Jackpot:  {}\n", money(&draw.jackpot)));
    out.push_str(&format!("Turnover: {}\n", money(&draw.turnover)));
    if !draw.prizes.is_empty() {
        out.push('\n');
        let rows: Vec<Vec<String>> = draw
            .prizes
            .iter()
            .map(|prize| {
                vec![
                    format!("Division {}", prize.division),
                    prize.winning_units.clone(),
                    money(&prize.dividend),
                ]
            })
            .collect();
        out.push_str(&table(&["PRIZE", "WINNING UNITS", "DIVIDEND"], &rows));
    }
    out.trim_end().to_string()
}

pub fn marksix_stats(stats: &MarkSixStatsView) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Mark Six frequency — {} draws ({} to {})\n\n",
        stats.draws_analysed,
        stats.from_draw.as_deref().unwrap_or("-"),
        stats.to_draw.as_deref().unwrap_or("-")
    ));
    out.push_str(&format!(
        "Odd {}  ·  Even {}  ·  Average sum {:.2}\n\n",
        stats.odd_even.odd, stats.odd_even.even, stats.average_sum
    ));
    let rows: Vec<Vec<String>> = stats
        .numbers
        .iter()
        .map(|stat| {
            vec![
                format!("{:02}", stat.number),
                stat.count.to_string(),
                format!("{:.1}%", stat.percentage),
                stat.last_seen_draw.as_deref().unwrap_or("-").to_string(),
            ]
        })
        .collect();
    out.push_str(&table(&["NO", "DRAWN", "SHARE", "LAST DRAW"], &rows));
    out.push_str("\nHot: ");
    out.push_str(
        &stats
            .hot
            .iter()
            .map(|stat| format!("{:02}({})", stat.number, stat.count))
            .collect::<Vec<_>>()
            .join("  "),
    );
    out.push_str("\nCold: ");
    out.push_str(
        &stats
            .cold
            .iter()
            .map(|stat| format!("{:02}({})", stat.number, stat.count))
            .collect::<Vec<_>>()
            .join("  "),
    );
    out.trim_end().to_string()
}

pub fn race_result(result: &RaceResultView) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} {} — Race {}{}\n",
        result.date,
        result.venue,
        result.race_no,
        if result.race_id.is_empty() {
            String::new()
        } else {
            format!(" ({})", result.race_id)
        }
    ));
    out.push_str(&format!("{}\n", result.race_name));
    out.push_str(&format!(
        "{} · {} · {} · {} · {} · {}\n\n",
        result.class,
        result.distance,
        result.going,
        result.course,
        result.prize,
        result.finish_time
    ));
    let rows: Vec<Vec<String>> = result
        .placings
        .iter()
        .map(|placing| {
            vec![
                placing.place.clone(),
                placing.horse_no.clone(),
                placing.horse.clone(),
                placing.jockey.clone(),
                placing.trainer.clone(),
                placing.actual_weight.clone(),
                placing.draw.clone(),
                placing.lengths_behind.clone(),
                placing.finish_time.clone(),
                placing.win_odds.clone(),
            ]
        })
        .collect();
    out.push_str(
        table(
            &[
                "PL", "NO", "HORSE", "JOCKEY", "TRAINER", "WT", "DR", "LBW", "TIME", "WIN",
            ],
            &rows,
        )
        .trim_end(),
    );
    if !result.dividends.is_empty() {
        out.push_str("\n\nDividends\n");
        let rows: Vec<Vec<String>> = result
            .dividends
            .iter()
            .map(|dividend| {
                vec![
                    dividend.pool.clone(),
                    dividend.combination.clone(),
                    money(&dividend.dividend),
                ]
            })
            .collect();
        out.push_str(table(&["POOL", "COMBINATION", "DIVIDEND"], &rows).trim_end());
    }
    if !result.incidents.is_empty() {
        out.push_str("\n\nIncidents\n");
        for incident in &result.incidents {
            out.push_str(&format!("  {}. {}\n", incident.horse_no, incident.incident));
        }
    }
    out.trim_end().to_string()
}

pub fn fixtures(fixtures: &[FixtureView]) -> String {
    if fixtures.is_empty() {
        return "No race meeting matches that range.".to_string();
    }
    let rows: Vec<Vec<String>> = fixtures
        .iter()
        .map(|fixture| {
            vec![
                fixture.date.clone(),
                fixture.day_of_week.clone(),
                fixture.venue.clone(),
                fixture.venue_code.clone(),
            ]
        })
        .collect();
    table(&["DATE", "DAY", "VENUE", "CODE"], &rows)
        .trim_end()
        .to_string()
}

fn had_odds(item: &FootballMatchView) -> (String, String, String) {
    let pool = item.odds.iter().find(|pool| pool.pool == "HAD");
    let combos = pool.and_then(|pool| pool.lines.first());
    let find = |label: &str| {
        combos
            .and_then(|line| line.odds.iter().find(|odd| odd.combination == label))
            .map(|odd| odd.odds.clone())
            .unwrap_or_default()
    };
    (find("H"), find("D"), find("A"))
}

pub fn football(matches: &[FootballMatchView]) -> String {
    if matches.is_empty() {
        return "No football match matches that filter.".to_string();
    }
    let rows: Vec<Vec<String>> = matches
        .iter()
        .map(|item| {
            let (home, draw, away) = had_odds(item);
            let score = if item.home_score.is_empty() && item.away_score.is_empty() {
                String::new()
            } else {
                format!("{}-{}", item.home_score, item.away_score)
            };
            vec![
                item.date.clone(),
                item.kick_off.clone(),
                format!("{} vs {}", item.home, item.away),
                score,
                item.tournament.clone(),
                item.status.clone(),
                home,
                draw,
                away,
            ]
        })
        .collect();
    table(
        &[
            "DATE",
            "TIME",
            "MATCH",
            "SCORE",
            "TOURNAMENT",
            "STATUS",
            "H",
            "D",
            "A",
        ],
        &rows,
    )
    .trim_end()
    .to_string()
}

pub fn football_match(item: &FootballMatchView) -> String {
    let mut out = String::new();
    out.push_str(&format!("{} vs {}\n", item.home, item.away));
    let score = if item.home_score.is_empty() && item.away_score.is_empty() {
        String::new()
    } else {
        format!("  {}-{}", item.home_score, item.away_score)
    };
    out.push_str(&format!(
        "{} {}  ·  {}{}  ·  {}\n\n",
        item.date, item.kick_off, item.tournament, score, item.status
    ));
    if item.odds.is_empty() {
        out.push_str("No odds are published for this match.\n");
        return out.trim_end().to_string();
    }
    for pool in &item.odds {
        out.push_str(&format!(
            "{} ({}) · {}\n",
            pool.name, pool.pool, pool.status
        ));
        for line in &pool.lines {
            if !line.condition.is_empty() {
                out.push_str(&format!("  line {}\n", line.condition));
            }
            let rows: Vec<Vec<String>> = line
                .odds
                .iter()
                .map(|odd| {
                    vec![
                        odd.combination.clone(),
                        odd.label.clone(),
                        odd.odds.clone(),
                        odd.status.clone(),
                    ]
                })
                .collect();
            out.push_str(&table(
                &["COMBINATION", "SELECTION", "ODDS", "STATUS"],
                &rows,
            ));
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}

fn wrap(value: &str, width: usize) -> String {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in value.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > width {
            lines.push(current.clone());
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines.join("\n")
}

pub fn profile(view: &ProfileView) -> String {
    let mut out = String::new();
    out.push_str(&format!("{} ({})\n", view.name, view.kind));
    if !view.age.is_empty() {
        out.push_str(&format!("Age: {}\n", view.age));
    }
    if !view.season.is_empty() {
        out.push_str(&format!("{}\n", view.season));
    }
    if !view.background.is_empty() {
        out.push('\n');
        out.push_str(&wrap(&view.background, 100));
        out.push('\n');
    }
    if !view.stats.is_empty() {
        out.push('\n');
        let rows: Vec<Vec<String>> = view
            .stats
            .iter()
            .map(|stat| vec![stat.label.clone(), stat.value.clone()])
            .collect();
        out.push_str(&table(&["STAT", "VALUE"], &rows));
    }
    if !view.form.is_empty() {
        out.push_str("\nRecent form\n");
        let rows: Vec<Vec<String>> = view
            .form
            .iter()
            .map(|run| {
                vec![
                    run.date.clone(),
                    run.place.clone(),
                    run.track.clone(),
                    run.distance.clone(),
                    run.going.clone(),
                    run.class.clone(),
                    run.draw.clone(),
                    run.rating.clone(),
                    run.jockey.clone(),
                    run.lengths_behind.clone(),
                    run.win_odds.clone(),
                    run.season.clone(),
                ]
            })
            .collect();
        out.push_str(
            table(
                &[
                    "DATE", "PL", "TRACK", "DIST", "G", "CLASS", "DR", "RTG", "JOCKEY", "LBW",
                    "WIN", "SEASON",
                ],
                &rows,
            )
            .trim_end(),
        );
    }
    out.push_str(&format!("\nsource: {}", view.source));
    out.trim_end().to_string()
}

pub fn lookup(hits: &[LookupHit]) -> String {
    if hits.is_empty() {
        return "No jockey, trainer or horse in the current meeting matches that name.".to_string();
    }
    let rows: Vec<Vec<String>> = hits
        .iter()
        .map(|hit| {
            vec![
                hit.kind.clone(),
                hit.name.clone(),
                hit.name_zh.clone(),
                hit.id.clone(),
                format!("R{}", hit.race_no),
                hit.horse.clone(),
            ]
        })
        .collect();
    table(&["KIND", "NAME", "中文名", "ID", "RACE", "HORSE"], &rows)
        .trim_end()
        .to_string()
}
