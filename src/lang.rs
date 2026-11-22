//! Output language for the human-readable CLI.

use std::str::FromStr;

/// Language used by the table and detail renderers.
///
/// JSON, CSV and MCP output stay in their stable data form; this only changes
/// the labels and prose printed for people.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    En,
    /// Traditional Chinese, as used by HKJC in Hong Kong.
    Zh,
}

impl Language {
    pub fn is_zh(self) -> bool {
        self == Self::Zh
    }

    /// Pick the English or Chinese string for the selected language.
    pub fn t(self, en: &'static str, zh: &'static str) -> &'static str {
        if self.is_zh() { zh } else { en }
    }

    /// Pick between borrowed English and Chinese values, falling back to
    /// English when the source did not provide a Chinese value.
    pub fn pick<'a>(self, en: &'a str, zh: &'a str) -> &'a str {
        if self.is_zh() && !zh.is_empty() {
            zh
        } else {
            en
        }
    }
}

impl FromStr for Language {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().replace('_', "-").as_str() {
            "en" | "en-hk" | "english" => Ok(Self::En),
            "zh" | "zh-hk" | "zh-hant" | "zh-tw" | "chinese" | "tc" => Ok(Self::Zh),
            other => Err(format!(
                "unsupported language {other:?}; use en or zh (Traditional Chinese)"
            )),
        }
    }
}

/// Traditional Chinese name for an HKJC venue code.
pub fn venue_name_zh(code: &str) -> String {
    match code {
        "ST" => "沙田".to_string(),
        "HV" => "跑馬地".to_string(),
        "CH" => "從化".to_string(),
        other if other.starts_with('S') && other.len() > 1 => format!("海外賽事 {other}"),
        "" => String::new(),
        other => other.to_string(),
    }
}

/// Translate the small set of kinds used by lookup and profile output.
pub fn kind_zh(kind: &str) -> &'static str {
    match kind.to_lowercase().as_str() {
        "jockey" | "jockeys" => "騎師",
        "trainer" | "trainers" => "練馬師",
        "horse" | "horses" => "馬匹",
        _ => "資料",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_language_aliases() {
        assert_eq!("en".parse::<Language>().unwrap(), Language::En);
        assert_eq!("zh-Hant".parse::<Language>().unwrap(), Language::Zh);
        assert_eq!("ZH_hk".parse::<Language>().unwrap(), Language::Zh);
        assert!("fr".parse::<Language>().is_err());
    }

    #[test]
    fn falls_back_when_chinese_is_missing() {
        assert_eq!(Language::Zh.pick("Sha Tin", ""), "Sha Tin");
        assert_eq!(Language::Zh.pick("Sha Tin", "沙田"), "沙田");
        assert_eq!(Language::En.pick("Sha Tin", "沙田"), "Sha Tin");
    }
}
