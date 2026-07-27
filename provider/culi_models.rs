//! CULI Model catalog — maps user-facing "CULI Model" names to Qveris model IDs
//! User sees: "CULI Flash", "CULI Pro", etc.
//! Underlying: Qveris API calls

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuliModel {
    /// User-facing ID (e.g. "culi-flash")
    pub id:           String,
    /// Display name shown in UI
    pub display_name: String,
    /// Short description
    pub description:  String,
    /// Underlying Qveris model ID
    pub qveris_model: String,
    /// Speed tier: fast | balanced | powerful
    pub tier:         String,
    /// Context window
    pub context:      u32,
}

pub fn culi_model_catalog() -> Vec<CuliModel> {
    vec![
        CuliModel {
            id:           "culi-auto".into(),
            display_name: "CULI Auto".into(),
            description:  "Smart routing — best model for the task".into(),
            qveris_model: "deepseek-v4-flash".into(),
            tier:         "auto".into(),
            context:      128_000,
        },
        CuliModel {
            id:           "culi-flash".into(),
            display_name: "CULI Flash".into(),
            description:  "Fast & efficient for everyday coding tasks".into(),
            qveris_model: "deepseek-v4-flash".into(),
            tier:         "fast".into(),
            context:      128_000,
        },
        CuliModel {
            id:           "culi-pro".into(),
            display_name: "CULI Pro".into(),
            description:  "Balanced quality for complex features".into(),
            qveris_model: "claude-fable-5".into(),
            tier:         "balanced".into(),
            context:      200_000,
        },
        CuliModel {
            id:           "culi-coder".into(),
            display_name: "CULI Coder".into(),
            description:  "Deep reasoning for architecture & algorithms".into(),
            qveris_model: "deepseek-r1".into(),
            tier:         "balanced".into(),
            context:      64_000,
        },
        CuliModel {
            id:           "culi-ultra".into(),
            display_name: "CULI Ultra".into(),
            description:  "Maximum capability for critical decisions".into(),
            qveris_model: "claude-opus-4-5".into(),
            tier:         "powerful".into(),
            context:      200_000,
        },
        CuliModel {
            id:           "culi-vision".into(),
            display_name: "CULI Vision".into(),
            description:  "Multimodal — code + image understanding".into(),
            qveris_model: "gemini-2.5-flash-image".into(),
            tier:         "balanced".into(),
            context:      128_000,
        },
    ]
}

/// Resolve a CULI model ID to the underlying Qveris model ID
/// Returns original ID if not a CULI model (pass-through)
pub fn resolve_culi_model(model_id: &str) -> String {
    culi_model_catalog()
        .into_iter()
        .find(|m| m.id == model_id)
        .map(|m| m.qveris_model)
        .unwrap_or_else(|| model_id.to_string())
}
