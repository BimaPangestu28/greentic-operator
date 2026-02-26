pub mod legacy_adapter_bridge;
pub mod secrets_bridge;
pub mod telemetry_bridge;

#[deprecated(note = "legacy adapter bridge; prefer engine::registry::Adapter")]
pub use legacy_adapter_bridge::AdapterBridge;
#[deprecated(
    note = "legacy adapter bridge helper; prefer typed engine::registry::Adapter impls"
)]
pub use legacy_adapter_bridge::FnAdapterBridge;
pub use secrets_bridge::FnSecretsHost;
pub use telemetry_bridge::FnTelemetryHost;
