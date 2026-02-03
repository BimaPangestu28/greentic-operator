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
