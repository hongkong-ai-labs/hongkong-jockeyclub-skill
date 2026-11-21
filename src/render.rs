//! Human-readable tables and detail views.

use crate::model::*;

fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|header| header.chars().count()).collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if index < widths.len() {
                widths[index] = widths[index].max(cell.chars().count());
            }
        }
    }
    let mut out = String::new();
    out.push_str(
        &headers
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
            &row.iter()
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

fn money(value: &str) -> String {
    if value.trim().is_empty() {
        return "-".to_string();
    }
    match value.trim().parse::<f64>() {
        Ok(amount) => format!("HK${amount:.0}"),
        Err(_) => value.to_string(),
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
                &["R", "TIME", "RACE", "CLASS", "DIST", "GOING", "RUNNERS", "STATUS"],
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
    out.push_str(&format!(
        "{}m · {} · {} · {} runners\n\n",
        race.distance,
        race.going,
        race.course,
        card.runners.len()
    ));
    let rows: Vec<Vec<String>> = card
        .runners
        .iter()
        .map(|runner| {
            vec![
                runner.horse_no.to_string(),
                runner.horse.clone(),
                runner.jockey.clone(),
                runner.trainer.clone(),
                runner.draw.clone(),
                runner.weight.clone(),
                runner.rating.clone(),
                runner.last_6_runs.clone(),
                runner.win_odds.clone(),
            ]
        })
        .collect();
    out.push_str(
        table(
            &["NO", "HORSE", "JOCKEY", "TRAINER", "DRAW", "WT", "RTG", "LAST 6", "WIN"],
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
        &["DRAW", "DATE", "NUMBERS", "+", "JACKPOT", "TURNOVER", "SNOWBALL"],
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
        stats
            .from_draw
            .map(|draw| draw.to_string())
            .unwrap_or_else(|| "-".to_string()),
        stats
            .to_draw
            .map(|draw| draw.to_string())
            .unwrap_or_else(|| "-".to_string())
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
                stat.last_seen_draw
                    .map(|draw| draw.to_string())
                    .unwrap_or_else(|| "-".to_string()),
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
        result.class, result.distance, result.going, result.course, result.prize, result.finish_time
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
            &["PL", "NO", "HORSE", "JOCKEY", "TRAINER", "WT", "DR", "LBW", "TIME", "WIN"],
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
        out.push_str(&table(&["POOL", "COMBINATION", "DIVIDEND"], &rows).trim_end());
    }
    if !result.incidents.is_empty() {
        out.push_str("\n\nIncidents\n");
        for incident in &result.incidents {
            out.push_str(&format!(
                "  {}. {}\n",
                incident.horse_no, incident.incident
            ));
        }
    }
    out.trim_end().to_string()
}
