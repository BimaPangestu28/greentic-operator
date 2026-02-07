use std::{io, sync::Once};

use tracing_subscriber::{filter::Directive, EnvFilter};

const DEFAULT_LOG_FILTER: &str = "greentic_secrets_repro=debug";
const ROUNDTRIP_FILTER: &str = "roundtrip=debug";
static INIT_LOGGER: Once = Once::new();

/// Initializes tracing once for the repro crate tests.
pub fn init_tracing() {
    INIT_LOGGER.call_once(|| {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER))
            .add_directive(ROUNDTRIP_FILTER.parse::<Directive>().unwrap());
        if let Err(err) = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_writer(io::stdout)
            .try_init()
        {
            eprintln!("greentic-secrets-repro: tracing initialization failed: {err}");
        }
    });
}

pub fn canonical_secret_uri(
    env: &str,
    tenant: &str,
    team: Option<&str>,
    provider: &str,
    key: &str,
) -> String {
    let team_segment = canonical_team_segment(team);
    let provider_segment = if provider.is_empty() {
        "messaging".to_string()
    } else {
        provider.to_string()
    };
    let normalized_key = secret_name::canonical_secret_name(key);
    format!(
        "secrets://{env}/{tenant}/{team}/{provider}/{key}",
        env = env,
        tenant = tenant,
        team = team_segment,
        provider = provider_segment,
        key = normalized_key
    )
}

fn canonical_team_segment(team: Option<&str>) -> String {
    team.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("default") {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
    .unwrap_or_else(|| "_".to_string())
}

mod secret_name {
    use std::{backtrace::Backtrace, env};

    #[inline]
    fn should_trace() -> bool {
        env::var("GREENTIC_SECRETS_TRACE").as_deref() == Ok("1")
    }

    fn trace_if_needed(raw: &str, canonical: &str) {
        if !should_trace() {
            return;
        }
        if raw != canonical {
            eprintln!(
                "GREENTIC_SECRETS_TRACE: canonicalizing secret name raw={raw} canonical={canonical}"
            );
            eprintln!("backtrace:\n{:?}", Backtrace::capture());
        }
    }

    #[inline]
    fn normalize_char(ch: char) -> Option<char> {
        match ch {
            'A'..='Z' => Some(ch.to_ascii_lowercase()),
            'a'..='z' | '0'..='9' | '_' => Some(ch),
            '-' | '.' | ' ' | '/' => Some('_'),
            _ => None,
        }
    }

    /// Convert a raw secret name (e.g. TELEGRAM_BOT_TOKEN) into the store-friendly canonical form.
    pub fn canonical_secret_name(raw: &str) -> String {
        let mut result = String::with_capacity(raw.len());
        let mut prev_underscore = false;

        for ch in raw.chars() {
            if let Some(normalized) = normalize_char(ch) {
                if normalized == '_' {
                    if prev_underscore {
                        continue;
                    }
                    prev_underscore = true;
                } else {
                    prev_underscore = false;
                }
                result.push(normalized);
            }
        }

        let trimmed = result.trim_matches('_').to_string();
        let canonical = if trimmed.is_empty() {
            "secret".to_string()
        } else {
            trimmed
        };
        trace_if_needed(raw, &canonical);
        canonical
    }
}
