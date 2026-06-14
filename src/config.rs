use serde::{Deserialize, Serialize};
use std::path::Path;

/// TOML config for gadgets, targets, voice prefs.
/// v1: simple. sane defaults. explicit. supports --config on cli, run, attach override.
/// load/save roundtrips. voice prefs (kaomoji_level) for future spam control in outputs.
/// targets for batch in `ghost run`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GhostConfig {
    /// Gadget names to load by default (e.g. ["poke", "roast"])
    #[serde(default)]
    pub gadgets: Vec<String>,

    /// Default dry-run for safety. User can override on CLI.
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,

    /// Personality flavor knobs (kaomoji spam level 0-10)
    #[serde(default)]
    pub voice: VoicePrefs,

    /// default targets for `ghost run --config` batch (or attach seeds)
    #[serde(default)]
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoicePrefs {
    /// more or less kaomoji spam in roasts/face. higher = more (¬‿¬) 💀
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

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let s = toml::to_string_pretty(self).map_err(|e| ConfigError::Parse(e.to_string()))?;
        std::fs::write(path, s).map_err(|e| ConfigError::Io(e.to_string()))?;
        Ok(())
    }

    /// Minimal default for first-run "just works" + voice sane
    pub fn with_defaults() -> Self {
        Self {
            gadgets: vec!["poke".into(), "roast".into()],
            dry_run: true,
            voice: VoicePrefs { kaomoji_level: 7 },
            targets: vec!["echo hi".into()],
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
        assert!(!cfg.targets.is_empty(), "targets in defaults for run batch");

        let serialized = toml::to_string(&cfg).expect("serialize");
        let back: GhostConfig = toml::from_str(&serialized).expect("roundtrip");
        assert_eq!(back.gadgets, cfg.gadgets);
        assert_eq!(back.targets, cfg.targets);
        assert_eq!(back.voice.kaomoji_level, 7);
    }

    // TDD integration-ish: on-disk save + load roundtrip (uses /tmp, cleans). voice prefs + targets persist.
    #[test]
    fn config_save_load_roundtrip_on_disk() {
        let mut cfg = GhostConfig::with_defaults();
        cfg.voice.kaomoji_level = 9;
        cfg.targets = vec!["ls /".into(), "echo test-target".into()];
        cfg.gadgets = vec!["poke".into(), "roast".into()];

        let p = "/tmp/ghost-test-config-roundtrip.toml";
        // ignore prior
        let _ = std::fs::remove_file(p);

        cfg.save(p).expect("save must work");
        let loaded = GhostConfig::load(p).expect("load after save");
        assert_eq!(loaded.voice.kaomoji_level, 9);
        assert_eq!(loaded.targets.len(), 2);
        assert!(loaded.targets.iter().any(|t| t.contains("test-target")));
        assert_eq!(loaded.gadgets, cfg.gadgets);

        let _ = std::fs::remove_file(p);
    }
}
