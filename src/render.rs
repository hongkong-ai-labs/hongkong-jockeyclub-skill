//! Human-readable tables and detail views.

use crate::lang::{Language, kind_zh, venue_name_zh};
use crate::model::*;

/// Terminal column width of one character. CJK ideographs and full-width
/// punctuation occupy two cells, which keeps Chinese tables aligned.
fn char_width(character: char) -> usize {
    let code = character as u32;
    let wide = matches!(code,
        0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1FAFF
    );
    if wide { 2 } else { 1 }
}

fn display_width(value: &str) -> usize {
    value.chars().map(char_width).sum()
}

fn pad(value: &str, width: usize) -> String {
    format!(
        "{value}{}",
        " ".repeat(width.saturating_sub(display_width(value)))
    )
}

fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|header| display_width(header)).collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if index < widths.len() {
                widths[index] = widths[index].max(display_width(cell));
            }
        }
    }
    let mut out = String::new();
    out.push_str(
        headers
            .iter()
            .enumerate()
            .map(|(index, header)| pad(header, widths[index]))
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
                    pad(&truncate(cell, width), width)
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
    if limit == 0 || display_width(value) <= limit {
        return value.to_string();
    }
    let mut width = 0;
    let mut out = String::new();
    for character in value.chars() {
        let next = char_width(character);
        if width + next + 1 > limit {
            break;
        }
        width += next;
        out.push(character);
    }
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

fn day_zh(day: &str) -> &'static str {
    match day.to_ascii_uppercase().as_str() {
        "MON" => "星期一",
        "TUE" => "星期二",
        "WED" => "星期三",
        "THU" => "星期四",
        "FRI" => "星期五",
        "SAT" => "星期六",
        "SUN" => "星期日",
        _ => "",
    }
}

fn football_status_zh(status: &str) -> String {
    match status {
        "Not started" => "未開賽".to_string(),
        "1st half" => "上半場".to_string(),
        "Half time" => "半場".to_string(),
        "2nd half" => "下半場".to_string(),
        "Finished" => "完場".to_string(),
        "Postponed" => "延期".to_string(),
        "Cancelled" => "取消".to_string(),
        "Void" => "賽事無效".to_string(),
        other => other.to_string(),
    }
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

pub fn meetings(meetings: &[MeetingView], lang: Language) -> String {
    if meetings.is_empty() {
        return lang
            .t(
                "No race meeting is currently published.",
                "目前未有公布賽事。",
            )
            .to_string();
    }
    let mut out = String::new();
    for (index, meeting) in meetings.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&format!(
            "{} — {} ({}){}\n",
            meeting.date,
            lang.pick(&meeting.venue, &venue_name_zh(&meeting.venue_code)),
            meeting.venue_code,
            if meeting.day_of_week.is_empty() {
                String::new()
            } else if lang.is_zh() && !day_zh(&meeting.day_of_week).is_empty() {
                format!("，{}", day_zh(&meeting.day_of_week))
            } else {
                format!(", {}", meeting.day_of_week)
            }
        ));
        out.push_str(&format!(
            "{} {}  ·  {} {}  ·  {} {}\n",
            meeting.total_races,
            lang.t("races", "場賽事"),
            lang.t("meeting", "賽事編號"),
            meeting.id,
            lang.t("status", "狀態"),
            meeting.status
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
                    lang.t("R", "場"),
                    lang.t("TIME", "時間"),
                    lang.t("RACE", "賽事"),
                    lang.t("CLASS", "班次"),
                    lang.t("DIST", "途程"),
                    lang.t("GOING", "場地"),
                    lang.t("RUNNERS", "出賽"),
                    lang.t("STATUS", "狀態"),
                ],
                &rows,
            )
            .trim_end(),
        );
        out.push('\n');
    }
    out.trim_end().to_string()
}

pub fn race_card(card: &RaceCardView, lang: Language) -> String {
    let race = &card.race;
    let mut out = String::new();
    let race_label = if lang.is_zh() {
        format!("第 {} 場", race.no)
    } else {
        format!("Race {}", race.no)
    };
    out.push_str(&format!(
        "{} {} — {} · {} {}{}\n",
        card.date,
        lang.pick(&card.venue, &venue_name_zh(&card.venue_code)),
        race_label,
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
        format!("{}{}", race.distance, lang.t("m", "米"))
    };
    let standby = card
        .runners
        .iter()
        .filter(|runner| {
            let status = runner.status.to_ascii_lowercase();
            status.contains("reserve") || status.contains("standby")
        })
        .count();
    out.push_str(&format!(
        "{} · {} · {} · {} {}{}\n\n",
        distance,
        race.going,
        race.course,
        card.runners.len().saturating_sub(standby),
        lang.t("runners", "匹出賽"),
        if standby > 0 {
            format!(" (+{standby} {})", lang.t("standby", "後備"))
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
                lang.pick(&runner.horse, &runner.horse_zh).to_string(),
                lang.pick(&runner.jockey, &runner.jockey_zh).to_string(),
                lang.pick(&runner.trainer, &runner.trainer_zh).to_string(),
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
                lang.t("NO", "馬號"),
                lang.t("HORSE", "馬名"),
                lang.t("JOCKEY", "騎師"),
                lang.t("TRAINER", "練馬師"),
                lang.t("DRAW", "檔位"),
                lang.t("WT", "負磅"),
                lang.t("RTG", "評分"),
                lang.t("LAST 6", "近六仗"),
                lang.t("WIN", "賠率"),
                lang.t("STATUS", "狀態"),
            ],
            &rows,
        )
        .trim_end(),
    );
    out
}

pub fn odds(view: &OddsView, lang: Language) -> String {
    if view.pools.is_empty() {
        return if lang.is_zh() {
            format!(
                "{} {} 第 {} 場暫無公布賠率。",
                view.date,
                venue_name_zh(&view.venue_code),
                view.race_no
            )
        } else {
            format!(
                "No odds are published for {} {} race {}.",
                view.date, view.venue, view.race_no
            )
        };
    }
    let mut out = String::new();
    let race = if lang.is_zh() {
        format!("第 {} 場", view.race_no)
    } else {
        format!("Race {}", view.race_no)
    };
    out.push_str(&format!(
        "{} {} — {} {}\n",
        view.date,
        lang.pick(&view.venue, &venue_name_zh(&view.venue_code)),
        race,
        lang.t("odds", "賠率")
    ));
    for pool in &view.pools {
        out.push_str(&format!(
            "\n{} ({}) · {} · {} {}\n",
            pool.name,
            pool.pool,
            pool.sell_status,
            lang.t("updated", "更新"),
            if pool.last_update.is_empty() {
                "-"
            } else {
                &pool.last_update
            }
        ));
        if pool.odds.is_empty() {
            out.push_str(if lang.is_zh() {
                "  暫無賠率\n"
            } else {
                "  no odds yet\n"
            });
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
        out.push_str(&table(
            &[lang.t("COMBINATION", "組合"), lang.t("ODDS", "賠率"), ""],
            &rows,
        ));
    }
    out.trim_end().to_string()
}

pub fn marksix(draws: &[MarkSixDrawView], lang: Language) -> String {
    if draws.is_empty() {
        return lang
            .t("No Mark Six draw found.", "未找到六合彩攪珠結果。")
            .to_string();
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
            lang.t("DRAW", "期"),
            lang.t("DATE", "日期"),
            lang.t("NUMBERS", "號碼"),
            lang.t("+", "特別"),
            lang.t("JACKPOT", "頭獎基金"),
            lang.t("TURNOVER", "投注總額"),
            lang.t("SNOWBALL", "多寶"),
        ],
        &rows,
    )
    .trim_end()
    .to_string()
}

pub fn marksix_draw(draw: &MarkSixDrawView, lang: Language) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} {} {} — {}\n\n",
        lang.t("Mark Six draw", "六合彩第"),
        draw.draw_no,
        lang.t("", "期"),
        draw.draw_date
    ));
    out.push_str(&format!(
        "{}: {}\n",
        lang.t("Numbers", "攪出號碼"),
        draw.numbers
            .iter()
            .map(|number| format!("{number:02}"))
            .collect::<Vec<_>>()
            .join("  ")
    ));
    if let Some(extra) = draw.extra {
        out.push_str(&format!("{}:   {extra:02}\n", lang.t("Extra", "特別號碼")));
    }
    if !draw.snowball.is_empty() {
        out.push_str(&format!(
            "{}: {}\n",
            lang.t("Snowball", "多寶"),
            draw.snowball
        ));
    }
    out.push_str(&format!(
        "{}:  {}\n",
        lang.t("Jackpot", "頭獎基金"),
        money(&draw.jackpot)
    ));
    out.push_str(&format!(
        "{}: {}\n",
        lang.t("Turnover", "投注總額"),
        money(&draw.turnover)
    ));
    if !draw.prizes.is_empty() {
        out.push('\n');
        let rows: Vec<Vec<String>> = draw
            .prizes
            .iter()
            .map(|prize| {
                vec![
                    if lang.is_zh() {
                        format!("第 {} 獎", prize.division)
                    } else {
                        format!("Division {}", prize.division)
                    },
                    prize.winning_units.clone(),
                    money(&prize.dividend),
                ]
            })
            .collect();
        out.push_str(&table(
            &[
                lang.t("PRIZE", "獎項"),
                lang.t("WINNING UNITS", "中獎單位"),
                lang.t("DIVIDEND", "派彩"),
            ],
            &rows,
        ));
    }
    out.trim_end().to_string()
}

pub fn marksix_stats(stats: &MarkSixStatsView, lang: Language) -> String {
    let mut out = String::new();
    if lang.is_zh() {
        out.push_str(&format!(
            "六合彩號碼頻率 — {} 期（{} 至 {}）\n\n",
            stats.draws_analysed,
            stats.from_draw.as_deref().unwrap_or("-"),
            stats.to_draw.as_deref().unwrap_or("-")
        ));
        out.push_str(&format!(
            "單 {}  ·  雙 {}  ·  平均總和 {:.2}\n\n",
            stats.odd_even.odd, stats.odd_even.even, stats.average_sum
        ));
    } else {
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
    }
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
    out.push_str(&table(
        &[
            lang.t("NO", "號碼"),
            lang.t("DRAWN", "次數"),
            lang.t("SHARE", "佔比"),
            lang.t("LAST DRAW", "最近出現"),
        ],
        &rows,
    ));
    out.push_str(lang.t("\nHot: ", "\n熱門："));
    out.push_str(
        &stats
            .hot
            .iter()
            .map(|stat| format!("{:02}({})", stat.number, stat.count))
            .collect::<Vec<_>>()
            .join("  "),
    );
    out.push_str(lang.t("\nCold: ", "\n冷門："));
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

pub fn race_result(result: &RaceResultView, lang: Language) -> String {
    let mut out = String::new();
    let race = if lang.is_zh() {
        format!("第 {} 場", result.race_no)
    } else {
        format!("Race {}", result.race_no)
    };
    out.push_str(&format!(
        "{} {} — {}{}\n",
        result.date,
        result.venue,
        race,
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
                lang.t("PL", "名次"),
                lang.t("NO", "馬號"),
                lang.t("HORSE", "馬名"),
                lang.t("JOCKEY", "騎師"),
                lang.t("TRAINER", "練馬師"),
                lang.t("WT", "負磅"),
                lang.t("DR", "檔"),
                lang.t("LBW", "距離"),
                lang.t("TIME", "時間"),
                lang.t("WIN", "賠率"),
            ],
            &rows,
        )
        .trim_end(),
    );
    if !result.dividends.is_empty() {
        out.push_str(lang.t("\n\nDividends\n", "\n\n派彩\n"));
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
        out.push_str(
            table(
                &[
                    lang.t("POOL", "彩池"),
                    lang.t("COMBINATION", "組合"),
                    lang.t("DIVIDEND", "派彩"),
                ],
                &rows,
            )
            .trim_end(),
        );
    }
    if !result.replays.is_empty() {
        out.push_str(lang.t("\n\nReplays\n", "\n\n重播\n"));
        for replay in &result.replays {
            out.push_str(&format!("  {}: {}\n", replay.label, replay.url));
        }
    }
    if !result.incidents.is_empty() {
        out.push_str(lang.t("\n\nIncidents\n", "\n\n事故\n"));
        for incident in &result.incidents {
            out.push_str(&format!("  {}. {}\n", incident.horse_no, incident.incident));
        }
    }
    out.trim_end().to_string()
}

pub fn fixtures(fixtures: &[FixtureView], lang: Language) -> String {
    if fixtures.is_empty() {
        return lang
            .t(
                "No race meeting matches that range.",
                "該日期範圍內沒有賽事。",
            )
            .to_string();
    }
    let rows: Vec<Vec<String>> = fixtures
        .iter()
        .map(|fixture| {
            vec![
                fixture.date.clone(),
                if lang.is_zh() && !day_zh(&fixture.day_of_week).is_empty() {
                    day_zh(&fixture.day_of_week).to_string()
                } else {
                    fixture.day_of_week.clone()
                },
                lang.pick(&fixture.venue, &venue_name_zh(&fixture.venue_code))
                    .to_string(),
                fixture.venue_code.clone(),
            ]
        })
        .collect();
    table(
        &[
            lang.t("DATE", "日期"),
            lang.t("DAY", "星期"),
            lang.t("VENUE", "馬場"),
            lang.t("CODE", "代號"),
        ],
        &rows,
    )
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

pub fn football(matches: &[FootballMatchView], lang: Language) -> String {
    if matches.is_empty() {
        return lang
            .t(
                "No football match matches that filter.",
                "沒有符合條件的足球賽事。",
            )
            .to_string();
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
                if lang.is_zh() {
                    format!("{} 對 {}", item.home_zh, item.away_zh)
                } else {
                    format!("{} vs {}", item.home, item.away)
                },
                score,
                lang.pick(&item.tournament, &item.tournament_zh).to_string(),
                if lang.is_zh() {
                    football_status_zh(&item.status)
                } else {
                    item.status.clone()
                },
                home,
                draw,
                away,
            ]
        })
        .collect();
    table(
        &[
            lang.t("DATE", "日期"),
            lang.t("TIME", "時間"),
            lang.t("MATCH", "賽事"),
            lang.t("SCORE", "比數"),
            lang.t("TOURNAMENT", "聯賽"),
            lang.t("STATUS", "狀態"),
            lang.t("H", "主"),
            lang.t("D", "和"),
            lang.t("A", "客"),
        ],
        &rows,
    )
    .trim_end()
    .to_string()
}

pub fn football_match(item: &FootballMatchView, lang: Language) -> String {
    let mut out = String::new();
    if lang.is_zh() {
        out.push_str(&format!("{} 對 {}\n", item.home_zh, item.away_zh));
    } else {
        out.push_str(&format!("{} vs {}\n", item.home, item.away));
    }
    let score = if item.home_score.is_empty() && item.away_score.is_empty() {
        String::new()
    } else {
        format!("  {}-{}", item.home_score, item.away_score)
    };
    out.push_str(&format!(
        "{} {}  ·  {}{}  ·  {}\n\n",
        item.date,
        item.kick_off,
        lang.pick(&item.tournament, &item.tournament_zh),
        score,
        if lang.is_zh() {
            football_status_zh(&item.status)
        } else {
            item.status.clone()
        }
    ));
    if item.odds.is_empty() {
        out.push_str(lang.t(
            "No odds are published for this match.\n",
            "此賽事暫無公布賠率。\n",
        ));
        return out.trim_end().to_string();
    }
    for pool in &item.odds {
        out.push_str(&format!(
            "{} ({}) · {}\n",
            pool.name, pool.pool, pool.status
        ));
        for line in &pool.lines {
            if !line.condition.is_empty() {
                out.push_str(&format!(
                    "  {} {}\n",
                    lang.t("line", "盤口"),
                    line.condition
                ));
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
                &[
                    lang.t("COMBINATION", "組合"),
                    lang.t("SELECTION", "選擇"),
                    lang.t("ODDS", "賠率"),
                    lang.t("STATUS", "狀態"),
                ],
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

pub fn profile(view: &ProfileView, lang: Language) -> String {
    let mut out = String::new();
    let kind = if lang.is_zh() {
        kind_zh(&view.kind).to_string()
    } else {
        view.kind.clone()
    };
    out.push_str(&format!("{} ({kind})\n", view.name));
    if !view.age.is_empty() {
        out.push_str(&format!("{}: {}\n", lang.t("Age", "年齡"), view.age));
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
        out.push_str(&table(
            &[lang.t("STAT", "統計"), lang.t("VALUE", "數值")],
            &rows,
        ));
    }
    if !view.form.is_empty() {
        out.push_str(lang.t("\nRecent form\n", "\n近期成績\n"));
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
                    lang.t("DATE", "日期"),
                    lang.t("PL", "名次"),
                    lang.t("TRACK", "跑道"),
                    lang.t("DIST", "途程"),
                    lang.t("G", "場地"),
                    lang.t("CLASS", "班次"),
                    lang.t("DR", "檔"),
                    lang.t("RTG", "評分"),
                    lang.t("JOCKEY", "騎師"),
                    lang.t("LBW", "距離"),
                    lang.t("WIN", "賠率"),
                    lang.t("SEASON", "馬季"),
                ],
                &rows,
            )
            .trim_end(),
        );
    }
    out.push_str(&format!(
        "\n{}: {}",
        lang.t("source", "資料來源"),
        view.source
    ));
    out.trim_end().to_string()
}

pub fn lookup(hits: &[LookupHit], lang: Language) -> String {
    if hits.is_empty() {
        return lang
            .t(
                "No jockey, trainer or horse in the current meeting matches that name.",
                "目前賽事沒有符合該名字的騎師、練馬師或馬匹。",
            )
            .to_string();
    }
    let rows: Vec<Vec<String>> = hits
        .iter()
        .map(|hit| {
            vec![
                if lang.is_zh() {
                    kind_zh(&hit.kind).to_string()
                } else {
                    hit.kind.clone()
                },
                hit.name.clone(),
                hit.name_zh.clone(),
                hit.id.clone(),
                format!("R{}", hit.race_no),
                hit.horse.clone(),
            ]
        })
        .collect();
    table(
        &[
            lang.t("KIND", "類別"),
            lang.t("NAME", "姓名"),
            "中文名",
            lang.t("ID", "編號"),
            lang.t("RACE", "場次"),
            lang.t("HORSE", "馬匹"),
        ],
        &rows,
    )
    .trim_end()
    .to_string()
}

pub fn standings(view: &StandingsView, lang: Language) -> String {
    if view.rows.is_empty() {
        return if lang.is_zh() {
            format!(
                "該選擇未有公布{}排行榜。",
                if view.kind.starts_with("jockey") {
                    "騎師"
                } else {
                    "練馬師"
                }
            )
        } else {
            format!(
                "No {} standings are published for that selection.",
                view.kind
            )
        };
    }
    let title = if lang.is_zh() {
        if view.kind.starts_with("jockey") {
            "騎師榜"
        } else {
            "練馬師榜"
        }
    } else if view.kind.starts_with("jockey") {
        "Jockeys' Ranking"
    } else {
        "Trainers' Ranking"
    };
    let course = if view.venue == "ALL" {
        lang.t("all racecourses", "全部馬場").to_string()
    } else {
        lang.pick(
            &crate::api::venue_name(&view.venue),
            &venue_name_zh(&view.venue),
        )
        .to_string()
    };
    let mut out = format!("{title} — {} ({course})\n\n", view.season);
    let rows: Vec<Vec<String>> = view
        .rows
        .iter()
        .map(|row| {
            vec![
                row.rank.to_string(),
                lang.pick(&row.name, &row.name_zh).to_string(),
                row.first.to_string(),
                row.second.to_string(),
                row.third.to_string(),
                row.fourth.to_string(),
                row.fifth.to_string(),
                row.starts.to_string(),
                format!("{:.1}%", row.win_rate),
                money(&row.stakes_won),
            ]
        })
        .collect();
    out.push_str(
        table(
            &[
                lang.t("#", "名次"),
                lang.t("NAME", "姓名"),
                lang.t("1ST", "冠"),
                lang.t("2ND", "亞"),
                lang.t("3RD", "季"),
                lang.t("4TH", "殿"),
                lang.t("5TH", "第五"),
                lang.t("RUNS", "出賽"),
                lang.t("WIN%", "勝出率"),
                lang.t("STAKES", "獎金"),
            ],
            &rows,
        )
        .trim_end(),
    );
    out
}

pub fn trials(view: &TrialsView, lang: Language) -> String {
    if view.batches.is_empty() {
        return if lang.is_zh() {
            format!("{} 未有公布試閘結果。", view.date)
        } else {
            format!("No barrier trials are published for {}.", view.date)
        };
    }
    let mut out = if lang.is_zh() {
        format!("試閘 — {}\n", view.date)
    } else {
        format!("Barrier trials — {}\n", view.date)
    };
    for batch in &view.batches {
        out.push_str(&format!(
            "\n{} · {} · {}\n",
            batch.batch, batch.venue, batch.distance
        ));
        let meta = [
            (!batch.going.is_empty())
                .then(|| format!("{}: {}", lang.t("Going", "場地"), batch.going)),
            (!batch.time.is_empty()).then(|| format!("{}: {}", lang.t("Time", "時間"), batch.time)),
            (!batch.sectional_time.is_empty()).then(|| {
                format!(
                    "{}: {}",
                    lang.t("Sectionals", "分段時間"),
                    batch.sectional_time
                )
            }),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        if !meta.is_empty() {
            out.push_str(&format!("{meta}\n"));
        }
        let rows: Vec<Vec<String>> = batch
            .horses
            .iter()
            .map(|horse| {
                vec![
                    horse.horse.clone(),
                    horse.jockey.clone(),
                    horse.trainer.clone(),
                    horse.draw.clone(),
                    horse.lengths_behind.clone(),
                    horse.running_position.clone(),
                    horse.time.clone(),
                    horse.result.clone(),
                    truncate(&horse.comment, 64),
                ]
            })
            .collect();
        out.push_str(
            table(
                &[
                    lang.t("HORSE", "馬名"),
                    lang.t("JOCKEY", "騎師"),
                    lang.t("TRAINER", "練馬師"),
                    lang.t("DRAW", "檔位"),
                    lang.t("LBW", "距離"),
                    lang.t("RUN", "走位"),
                    lang.t("TIME", "時間"),
                    lang.t("RESULT", "結果"),
                    lang.t("COMMENT", "評語"),
                ],
                &rows,
            )
            .trim_end(),
        );
        out.push('\n');
    }
    out.trim_end().to_string()
}

pub fn sectionals(view: &SectionalsView, lang: Language) -> String {
    if view.horses.is_empty() {
        return lang
            .t(
                "No sectional times are published for that race.",
                "該賽事未有公布分段時間。",
            )
            .to_string();
    }
    let sections = view
        .horses
        .iter()
        .map(|horse| horse.splits.len())
        .max()
        .unwrap_or(0);
    let mut out = if lang.is_zh() {
        format!("分段時間 — {} 第 {} 場\n", view.date, view.race_no)
    } else {
        format!("Sectional times — {} race {}\n", view.date, view.race_no)
    };
    if !view.race_times.is_empty() {
        out.push_str(&format!(
            "{}: {}\n",
            lang.t("Race time", "賽事時間"),
            view.race_times.join(" · ")
        ));
    }
    if !view.sectional_times.is_empty() {
        out.push_str(&format!(
            "{}: {}\n",
            lang.t("Sectionals", "分段時間"),
            view.sectional_times.join(" · ")
        ));
    }
    out.push('\n');
    let mut headers = vec![
        lang.t("PL", "名次").to_string(),
        lang.t("NO", "馬號").to_string(),
        lang.t("HORSE", "馬名").to_string(),
    ];
    headers.extend((1..=sections).map(|index| format!("S{index}")));
    headers.push(lang.t("TIME", "時間").to_string());
    let rows: Vec<Vec<String>> = view
        .horses
        .iter()
        .map(|horse| {
            let mut row = vec![
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
                        if split.margin.is_empty() {
                            position
                        } else {
                            format!("{position} {}", split.margin)
                        }
                    }
                    None => String::new(),
                });
            }
            row.push(horse.finish_time.clone());
            row
        })
        .collect();
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    out.push_str(table(&header_refs, &rows).trim_end());
    out
}

pub fn pace(view: &SectionalsView, lang: Language) -> String {
    if view.horses.is_empty() {
        return lang
            .t(
                "No sectional times are published for that race.",
                "該賽事未有公布分段時間。",
            )
            .to_string();
    }
    let sections = view
        .horses
        .iter()
        .map(|horse| horse.splits.len())
        .max()
        .unwrap_or(0);
    let mut out = if lang.is_zh() {
        format!("步速圖 — {} 第 {} 場\n", view.date, view.race_no)
    } else {
        format!("Pace map — {} race {}\n", view.date, view.race_no)
    };
    if !view.sectional_times.is_empty() {
        out.push_str(&format!(
            "{}: {}\n",
            lang.t("Sectionals", "分段時間"),
            view.sectional_times.join(" · ")
        ));
    }
    out.push('\n');
    let mut headers = vec![
        lang.t("PL", "名次").to_string(),
        lang.t("NO", "馬號").to_string(),
        lang.t("HORSE", "馬名").to_string(),
    ];
    headers.extend((1..=sections).map(|index| format!("S{index}")));
    headers.push(lang.t("FIN", "完成").to_string());
    let rows: Vec<Vec<String>> = view
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
        .collect();
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    out.push_str(table(&header_refs, &rows).trim_end());
    out
}

pub fn marksix_check(view: &MarkSixCheckView, lang: Language) -> String {
    let ticket = view
        .ticket
        .iter()
        .map(|number| format!("{number:02}"))
        .collect::<Vec<_>>()
        .join(" ");
    let drawn = view
        .drawn
        .iter()
        .map(|number| format!("{number:02}"))
        .collect::<Vec<_>>()
        .join(" ");
    let extra = view
        .extra
        .map(|number| format!("{number:02}"))
        .unwrap_or_default();
    let matches = view
        .matches
        .iter()
        .map(|number| format!("{number:02}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = if lang.is_zh() {
        format!("六合彩 — {} 第 {} 期\n\n", view.draw_date, view.draw_no)
    } else {
        format!("Mark Six — {} draw {}\n\n", view.draw_date, view.draw_no)
    };
    out.push_str(&format!("{}    {ticket}\n", lang.t("Ticket", "彩票")));
    out.push_str(&format!(
        "{}     {drawn}  + {extra}\n",
        lang.t("Drawn", "攪出")
    ));
    out.push_str(&format!(
        "{}   {}{}\n",
        lang.t("Matched", "中獎號碼"),
        if matches.is_empty() { "-" } else { &matches },
        if view.extra_matched {
            lang.t(" + extra", " + 特別號碼")
        } else {
            ""
        }
    ));
    let message = if lang.is_zh() {
        match view.division {
            Some(division) => format!("第 {division} 獎"),
            None => "未中獎".to_string(),
        }
    } else {
        view.message.clone()
    };
    out.push_str(&format!("{}    {message}", lang.t("Result", "結果")));
    if !view.dividend.is_empty() {
        out.push_str(&format!(
            " ({} {})",
            money(&view.dividend),
            lang.t("per unit", "每注")
        ));
    }
    out
}

/// iCalendar feed for race meetings, suitable for `fixtures --ics > races.ics`.
pub fn fixtures_ics(fixtures: &[FixtureView], lang: Language) -> String {
    let mut out = String::from(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//hongkong-jockeyclub-skill//HKJC fixtures//EN\r\nCALSCALE:GREGORIAN\r\n",
    );
    for fixture in fixtures {
        let date = fixture.date.replace('-', "");
        let summary = if lang.is_zh() {
            format!(
                "香港賽馬 — {} ({})",
                venue_name_zh(&fixture.venue_code),
                fixture.venue_code
            )
        } else {
            format!("HKJC races — {} ({})", fixture.venue, fixture.venue_code)
        };
        out.push_str("BEGIN:VEVENT\r\n");
        out.push_str(&format!(
            "UID:{}-{}@hongkong-jockeyclub-skill\r\n",
            date, fixture.venue_code
        ));
        out.push_str(&format!("DTSTAMP:{date}T000000Z\r\n"));
        out.push_str(&format!("DTSTART;VALUE=DATE:{date}\r\n"));
        out.push_str(&format!("SUMMARY:{summary}\r\n"));
        out.push_str(&format!(
            "LOCATION:{}\r\n",
            lang.pick(&fixture.venue, &venue_name_zh(&fixture.venue_code))
        ));
        out.push_str("END:VEVENT\r\n");
    }
    out.push_str("END:VCALENDAR\r\n");
    out
}

pub fn history(view: &HistoryView, lang: Language) -> String {
    if view.entries.is_empty() {
        return if lang.is_zh() {
            format!(
                "{} {} 第 {} 場未有賠率紀錄。",
                view.meeting_date,
                venue_name_zh(&view.venue),
                view.race_no
            )
        } else {
            format!(
                "No odds history recorded for {} {} race {}.",
                view.meeting_date, view.venue, view.race_no
            )
        };
    }
    let snapshots = if lang.is_zh() {
        format!("{}", view.snapshots)
    } else if view.snapshots == 1 {
        String::new()
    } else {
        "s".to_string()
    };
    let mut out = if lang.is_zh() {
        format!(
            "賠率紀錄 — {} {} 第 {} 場（{} 個紀錄）\n\n",
            view.meeting_date,
            venue_name_zh(&view.venue),
            view.race_no,
            view.snapshots
        )
    } else {
        format!(
            "Odds history — {} {} race {} ({} snapshot{})\n\n",
            view.meeting_date, view.venue, view.race_no, view.snapshots, snapshots
        )
    };
    let rows: Vec<Vec<String>> = view
        .entries
        .iter()
        .map(|entry| {
            vec![
                entry.pool.clone(),
                entry.combination.clone(),
                entry.first.clone(),
                entry.last.clone(),
                format!("{:+.2}", entry.movement),
                entry.points.len().to_string(),
            ]
        })
        .collect();
    out.push_str(
        table(
            &[
                lang.t("POOL", "彩池"),
                lang.t("COMBINATION", "組合"),
                lang.t("FIRST", "首次"),
                lang.t("LAST", "最新"),
                lang.t("MOVE", "變動"),
                lang.t("POINTS", "紀錄"),
            ],
            &rows,
        )
        .trim_end(),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_columns_use_terminal_width() {
        let table = table(
            &["馬名", "賠率"],
            &[vec!["潘頓".to_string(), "2.5".to_string()]],
        );
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines[0], "馬名  賠率");
        assert_eq!(lines[1], "----  ----");
        assert_eq!(lines[2], "潘頓  2.5");
    }

    #[test]
    fn chinese_ticket_result_uses_the_division() {
        let view = MarkSixCheckView {
            draw_date: "2026-11-23".to_string(),
            draw_no: 1,
            ticket: vec![1, 2, 3, 4, 5, 6],
            drawn: vec![1, 2, 3, 10, 11, 12],
            extra: Some(7),
            matches: vec![1, 2, 3],
            extra_matched: false,
            division: Some(7),
            dividend: "10".to_string(),
            message: "Division 7 winner".to_string(),
        };
        let output = marksix_check(&view, Language::Zh);
        assert!(output.contains("第 7 獎"), "{output}");
        assert!(output.contains("每注"), "{output}");
    }

    #[test]
    fn chinese_football_status_is_translated() {
        assert_eq!(football_status_zh("2nd half"), "下半場");
        assert_eq!(football_status_zh("something new"), "something new");
    }
}
