use std::env;

use greentic_i18n as shared;
pub use greentic_i18n::I18nText;

pub fn select_locale(explicit: Option<&str>) -> String {
    let cli_locale = env::var("GREENTIC_LOCALE_CLI").ok();
    let env_locale = env::var("GREENTIC_LOCALE").ok();
    let system = system_locale();
    shared::select_locale_with_sources(
        cli_locale.as_deref(),
        explicit,
        env_locale.as_deref(),
        system.as_deref(),
    )
}

pub fn resolve_message(key: &str, fallback: &str, locale: &str) -> String {
    shared::resolve_message(key, fallback, locale)
}

pub fn resolve_text(text: &I18nText, locale: &str) -> String {
    shared::resolve_text(text, locale)
}

fn system_locale() -> Option<String> {
    for key in ["LC_ALL", "LANG", "LC_MESSAGES"] {
        if let Ok(value) = env::var(key) {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                continue;
            }
            let stripped = trimmed.split('.').next().unwrap_or(trimmed);
            if !stripped.is_empty() {
                return Some(shared::normalize_locale(stripped));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_message_uses_fallback_for_unknown_key() {
        let message = resolve_message("runner.unknown", "fallback message", "en");
        assert_eq!(message, "fallback message");
    }

    #[test]
    fn resolve_text_uses_key_and_fallback() {
        let text = I18nText::new("runner.operator.op_not_found", "fallback");
        let message = resolve_text(&text, "en");
        assert_eq!(message, "operation not found");
    }

    #[test]
    fn select_locale_prefers_explicit_over_env_and_system() {
        assert_eq!(
            shared::select_locale_with_sources(
                None,
                Some("en-US"),
                Some("fr-FR"),
                Some("nl_NL.UTF-8")
            ),
            "en"
        );
    }

    #[test]
    fn select_locale_prefers_cli_override() {
        assert_eq!(
            shared::select_locale_with_sources(
                Some("it-IT"),
                Some("en-US"),
                Some("fr-FR"),
                Some("nl_NL.UTF-8")
            ),
            "it"
        );
    }

    #[test]
    fn select_locale_uses_env_over_system() {
        assert_eq!(
            shared::select_locale_with_sources(None, None, Some("de-DE"), Some("nl_NL.UTF-8")),
            "de"
        );
    }

    #[test]
    fn select_locale_falls_back_to_system_then_en() {
        assert_eq!(
            shared::select_locale_with_sources(None, None, None, Some("es_ES.UTF-8")),
            "es"
        );
        assert_eq!(
            shared::select_locale_with_sources(None, None, None, None),
            "en"
        );
    }
}
