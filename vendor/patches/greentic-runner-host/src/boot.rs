use anyhow::Result;

use crate::TelemetryCfg;
use crate::http::health::HealthState;

#[cfg(feature = "telemetry")]
use greentic_telemetry::init_telemetry_from_config;
#[cfg(feature = "telemetry")]
use tracing::info;

/// Initialise host-level subsystems (telemetry, health markers).
pub fn init(health: &HealthState, _telemetry: Option<&TelemetryCfg>) -> Result<()> {
    #[cfg(feature = "telemetry")]
    if let Some(cfg) = _telemetry {
        info!(
            service = cfg.config.service_name,
            "initialising telemetry pipeline"
        );
        init_telemetry_from_config(cfg.config.clone(), cfg.export.clone())?;
    }

    health.set_ready();
    Ok(())
}
