//! Embedded UI translations. Git output and repository content are never translated.
use std::{collections::HashMap, sync::OnceLock};

pub const LANGUAGES: &[&str] = &["en", "ja", "ko", "nl", "pl", "pt", "ru", "zh-CN", "zh-TW"];
static ACTIVE: OnceLock<Translator> = OnceLock::new();

pub struct Translator {
    pub language: &'static str,
    messages: HashMap<String, String>,
}

impl Translator {
    pub fn new(language: &'static str) -> Self {
        let json = match language {
            "ja" => include_str!("../locales/ja.json"),
            "ko" => include_str!("../locales/ko.json"),
            "nl" => include_str!("../locales/nl.json"),
            "pl" => include_str!("../locales/pl.json"),
            "pt" => include_str!("../locales/pt.json"),
            "ru" => include_str!("../locales/ru.json"),
            "zh-CN" => include_str!("../locales/zh-CN.json"),
            "zh-TW" => include_str!("../locales/zh-TW.json"),
            _ => "{}",
        };
        Self {
            language,
            messages: serde_json::from_str(json).expect("invalid embedded translation catalog"),
        }
    }

    pub fn text<'a>(&'a self, source: &'a str) -> &'a str {
        self.messages
            .get(source)
            .filter(|s| !s.is_empty())
            .map_or(source, String::as_str)
    }
}

/// Normalize common OS locale names while distinguishing simplified/traditional Chinese.
pub fn normalize(locale: &str) -> Option<&'static str> {
    let base = locale
        .split(['.', '@'])
        .next()
        .unwrap_or(locale)
        .replace('_', "-")
        .to_ascii_lowercase();
    if base == "c" || base == "posix" {
        return Some("en");
    }
    let parts: Vec<_> = base.split('-').collect();
    match parts.first().copied()? {
        "zh" => Some(
            if parts.contains(&"hant")
                || (!parts.contains(&"hans")
                    && parts.iter().any(|p| matches!(*p, "tw" | "hk" | "mo")))
            {
                "zh-TW"
            } else {
                "zh-CN"
            },
        ),
        language => LANGUAGES.iter().copied().find(|code| *code == language),
    }
}

/// Explicit unsupported choices are errors; unsupported system locales use English.
pub fn resolve(
    configured: &str,
    env: impl Fn(&str) -> Option<String>,
) -> anyhow::Result<&'static str> {
    if configured != "auto" {
        return normalize(configured).ok_or_else(|| {
            anyhow::anyhow!(
                "Unsupported UI language: {configured}. Use auto or {}",
                LANGUAGES.join(", ")
            )
        });
    }
    for variable in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(value) = env(variable).filter(|v| !v.is_empty()) {
            return Ok(normalize(&value).unwrap_or("en"));
        }
    }
    Ok("en")
}

/// Call once at startup, before creating any UI or worker threads.
pub fn init(configured: &str) -> anyhow::Result<()> {
    let language = resolve(configured, |name| std::env::var(name).ok())?;
    ACTIVE
        .set(Translator::new(language))
        .map_err(|_| anyhow::anyhow!("UI language already initialized"))
}

pub fn language() -> &'static str {
    ACTIVE.get().map_or("en", |t| t.language)
}

pub fn text(source: &'static str) -> &'static str {
    ACTIVE.get().map_or(source, |t| t.text(source))
}

/// Interpolate once, so braces inside a filename or message are never substituted.
pub fn interpolate(template: &str, args: &[(&str, String)]) -> String {
    let mut result = String::with_capacity(template.len());
    let mut tail = template;
    while let Some(start) = tail.find('{') {
        result.push_str(&tail[..start]);
        let Some(end) = tail[start..].find('}') else {
            result.push_str(&tail[start..]);
            return result;
        };
        let end = start + end;
        let name = &tail[start + 1..end];
        if let Some((_, value)) = args.iter().find(|(key, _)| *key == name) {
            result.push_str(value);
        } else {
            result.push_str(&tail[start..=end]);
        }
        tail = &tail[end + 1..];
    }
    result.push_str(tail);
    result
}

#[macro_export]
macro_rules! tr {
    ($source:literal) => {
        $crate::i18n::text($source)
    };
}

#[macro_export]
macro_rules! trf {
    ($source:literal $(, $name:ident = $value:expr)* $(,)?) => {
        $crate::i18n::interpolate($crate::i18n::text($source), &[$((stringify!($name), ($value).to_string())),*])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_locale_precedence_and_variants() {
        for (input, expected) in [
            ("ja_JP.UTF-8", "ja"),
            ("pt_BR", "pt"),
            ("zh_Hant", "zh-TW"),
            ("zh_HK", "zh-TW"),
            ("zh_SG", "zh-CN"),
            ("C.UTF-8", "en"),
        ] {
            assert_eq!(normalize(input), Some(expected));
        }
        assert_eq!(
            resolve("auto", |k| match k {
                "LC_ALL" => Some("ko_KR".into()),
                "LANG" => Some("ja_JP".into()),
                _ => None,
            })
            .unwrap(),
            "ko"
        );
        assert_eq!(resolve("auto", |_| Some("xx_XX".into())).unwrap(), "en");
        assert_eq!(resolve("ja", |_| Some("en_US".into())).unwrap(), "ja");
        assert!(resolve("xx", |_| None).is_err());
    }

    #[test]
    fn substitution_preserves_repository_data_and_allows_reordering() {
        assert_eq!(
            interpolate(
                "{second}: {first}",
                &[
                    ("first", "{second}日本語.txt".into()),
                    ("second", "value".into())
                ]
            ),
            "value: {second}日本語.txt"
        );
        assert_eq!(
            Translator::new("ja").text("Missing translation"),
            "Missing translation"
        );
    }
}
