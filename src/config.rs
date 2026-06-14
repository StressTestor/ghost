use serde::{Deserialize, Serialize};
use std::path::Path;

/// TOML config for gadgets, targets, voice prefs.
/// v1: simple. sane defaults. explicit.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GhostConfig {
    /// Gadget names to load by default (e.g. ["poke", "roast"])
    #[serde(default)]
    pub gadgets: Vec<String>,

    /// Default dry-run for safety. User can override on CLI.
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,

    /// Personality flavor knobs (future: intensity, allowed_kaomoji etc)
    #[serde(default)]
    pub voice: VoicePrefs,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoicePrefs {
    /// more or less kaomoji spam
    #[serde(default)]
    pub kaomoji_level: u8, // 0-10
}

fn default_dry_run() -> bool {
    true
}

impl GhostConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let p = path.as_ref();
        if !p.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(p).map_err(|e| ConfigError::Io(e.to_string()))?;
        let cfg: Self = toml::from_str(&contents).map_err(|e| ConfigError::Parse(e.to_string()))?;
        Ok(cfg)
    }

    /// Minimal default for first-run "just works"
    pub fn with_defaults() -> Self {
        Self {
            gadgets: vec!["poke".into(), "roast".into()],
            dry_run: true,
            voice: VoicePrefs { kaomoji_level: 7 },
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("io error reading config: {0}")]
    Io(String),
    #[error("parse error in toml: {0}")]
    Parse(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    // skeleton test: default + toml roundtrip only. no fs, no extra deps.
    #[test]
    fn config_defaults_and_roundtrip() {
        let cfg = GhostConfig::with_defaults();
        assert!(cfg.dry_run);
        assert!(cfg.gadgets.contains(&"poke".to_string()));

        let serialized = toml::to_string(&cfg).expect("serialize");
        let back: GhostConfig = toml::from_str(&serialized).expect("roundtrip");
        assert_eq!(back.gadgets, cfg.gadgets);
    }
}
