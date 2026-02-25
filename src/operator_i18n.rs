use anyhow::Context;
use include_dir::{Dir, include_dir};
use once_cell::sync::Lazy;
use std::collections::BTreeMap;
use std::sync::RwLock;

pub type Map = BTreeMap<String, String>;

static OPERATOR_CLI_I18N: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/i18n/operator_cli");
static CURRENT_LOCALE: Lazy<RwLock<String>> = Lazy::new(|| RwLock::new("en".to_string()));

pub fn select_locale(cli_locale: Option<&str>) -> String {
    let env_locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|key| std::env::var(key).ok())
        .filter(|value| {
            let trimmed = value.trim();
            !trimmed.is_empty() && trimmed != "C" && trimmed != "POSIX"
        });
    greentic_i18n::select_locale_with_sources(cli_locale, None, env_locale.as_deref(), None)
}

pub fn set_locale(locale: impl Into<String>) {
    let normalized = greentic_i18n::normalize_locale(&locale.into());
    if let Ok(mut guard) = CURRENT_LOCALE.write() {
        *guard = normalized;
    }
}

pub fn current_locale() -> String {
    CURRENT_LOCALE
        .read()
        .map(|value| value.clone())
        .unwrap_or_else(|_| "en".to_string())
}

pub fn tr(key: &str, fallback: &str) -> String {
    tr_for_locale(key, fallback, &current_locale())
}

pub fn trf(key: &str, fallback: &str, args: &[&str]) -> String {
    let mut rendered = tr(key, fallback);
    for value in args {
        rendered = rendered.replacen("{}", value, 1);
    }
    rendered
}

pub fn tr_for_locale(key: &str, fallback: &str, locale: &str) -> String {
    match load_cli(locale) {
        Ok(map) => map
            .get(key)
            .cloned()
            .unwrap_or_else(|| fallback.to_string()),
        Err(_) => fallback.to_string(),
    }
}

pub fn load_cli(locale: &str) -> anyhow::Result<Map> {
    for candidate in locale_candidates(locale) {
        if let Some(file) = OPERATOR_CLI_I18N.get_file(&candidate) {
            let raw = file.contents_utf8().ok_or_else(|| {
                anyhow::anyhow!("operator cli i18n file is not valid UTF-8: {candidate}")
            })?;
            return serde_json::from_str(raw)
                .with_context(|| format!("parse embedded operator cli i18n map {candidate}"));
        }
    }
    Ok(Map::new())
}

fn locale_candidates(locale: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut push_candidate = |candidate: String| {
        if !out.iter().any(|existing| existing == &candidate) {
            out.push(candidate);
        }
    };
    let trimmed = locale.trim();
    if !trimmed.is_empty() {
        push_candidate(format!("{}.json", trimmed));
        let primary = greentic_i18n::normalize_locale(trimmed);
        push_candidate(format!("{}.json", primary));
    }
    push_candidate("en.json".to_string());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_requested_locale_before_english() {
        let map = load_cli("de-DE").expect("load de locale");
        assert_eq!(
            map.get("cli.common.answer_yes_no").map(String::as_str),
            Some("bitte mit y oder n antworten")
        );
    }
}
