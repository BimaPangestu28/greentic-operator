use crate::cache::engine_profile::{CpuPolicy, EngineProfile};

#[test]
fn engine_profile_id_is_stable_for_default_config() {
    let engine = wasmtime::Engine::default();
    let profile = EngineProfile::from_engine(&engine, CpuPolicy::Native, "default".to_string());
    let profile_again =
        EngineProfile::from_engine(&engine, CpuPolicy::Native, "default".to_string());
    assert_eq!(profile.id(), profile_again.id());
}
