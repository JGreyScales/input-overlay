use serde::{Deserialize, Serialize};
use std::path::Path;

//bumped when a default changes in a way that also has to reach configs written by older builds
pub const CONFIG_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub auth_token: String,
    pub key_whitelist: Vec<String>,
    pub raw_mouse_min_delta: i32,
    pub send_mouse_move: bool,
    pub cpu_affinity: Vec<u32>,
    pub analog_keyboard: String,
    pub gamepad_device: String,
    pub http_enabled: bool,
    pub http_port: u16,
    pub linux_evdev_keyboard_device: String,
    pub linux_raw_mouse_device: String,
    pub dismissed_update_versions: Vec<String>,
    pub theme: Option<String>,
    pub flush_hz: u32,
    //files from before this field existed read as 0, thats how migrate() tells them apart
    #[serde(default)]
    pub config_version: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 4455,
            auth_token: String::new(),
            key_whitelist: Vec::new(),
            raw_mouse_min_delta: 0,
            send_mouse_move: true,
            cpu_affinity: Vec::new(),
            analog_keyboard: String::new(),
            gamepad_device: String::new(),
            http_enabled: false,
            http_port: 4456,
            linux_evdev_keyboard_device: String::new(),
            linux_raw_mouse_device: String::new(),
            dismissed_update_versions: Vec::new(),
            theme: None,
            flush_hz: 125,
            config_version: CONFIG_VERSION,
        }
    }
}

pub fn load(path: &Path) -> Config {
    if path.exists() {
        match std::fs::read_to_string(path) {
            Ok(s) => match parse(&s) {
                Ok(cfg) => return cfg,
                Err(e) => tracing::error!("config parse error: {e}"),
            },
            Err(e) => tracing::error!("config read error: {e}"),
        }
    }
    Config::default()
}

fn parse(s: &str) -> serde_json::Result<Config> {
    let mut cfg = serde_json::from_str::<Config>(s.trim_start_matches('\u{feff}'))?;
    migrate(&mut cfg);
    Ok(cfg)
}

//the first start saves the config (to keep the generated auth token), so the old default of pinning the whole process
//to cores 0 and 1 is sitting in every existing config file and changing the default alone would never reach those installs.
//a pin thats typed into the settings from now on gets saved together with the current version and is left alone
fn migrate(cfg: &mut Config) {
    if cfg.config_version < 1 && cfg.cpu_affinity == [0, 1] {
        tracing::info!("config: dropping the old default cpu affinity (cores 0 and 1)");
        cfg.cpu_affinity.clear();
    }
    cfg.config_version = CONFIG_VERSION;
}

pub fn save(path: &Path, config: &Config) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(config).expect("config serialize failed");
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_config_does_not_pin_any_cores() {
        let cfg = Config::default();
        assert!(cfg.cpu_affinity.is_empty());
        assert_eq!(cfg.config_version, CONFIG_VERSION);
    }

    #[test]
    fn the_old_default_pin_is_dropped_from_configs_written_by_older_builds() {
        let cfg = parse(r#"{"port": 4455, "auth_token": "abc", "cpu_affinity": [0, 1], "flush_hz": 125}"#).unwrap();
        assert!(cfg.cpu_affinity.is_empty());
        assert_eq!(cfg.config_version, CONFIG_VERSION);
        assert_eq!(cfg.auth_token, "abc");
    }

    #[test]
    fn a_pin_that_was_not_the_old_default_is_kept() {
        let cfg = parse(r#"{"cpu_affinity": [2, 3]}"#).unwrap();
        assert_eq!(cfg.cpu_affinity, [2, 3]);
        let cfg = parse(r#"{"cpu_affinity": [1, 0]}"#).unwrap();
        assert_eq!(cfg.cpu_affinity, [1, 0]);
    }

    #[test]
    fn cores_0_and_1_chosen_after_the_change_are_kept() {
        let chosen = Config { cpu_affinity: vec![0, 1], ..Config::default() };
        let cfg = parse(&serde_json::to_string_pretty(&chosen).unwrap()).unwrap();
        assert_eq!(cfg.cpu_affinity, [0, 1]);
    }

    #[test]
    fn a_config_without_the_field_gets_the_new_default() {
        let cfg = parse(r#"{"port": 4460}"#).unwrap();
        assert!(cfg.cpu_affinity.is_empty());
        assert_eq!(cfg.port, 4460);
    }

    #[test]
    fn a_saved_config_reads_back_the_same() {
        let saved = Config { auth_token: "t".into(), key_whitelist: vec!["a".into()], ..Config::default() };
        let text = serde_json::to_string_pretty(&saved).unwrap();
        let back = parse(&format!("\u{feff}{text}")).unwrap();
        assert_eq!(serde_json::to_value(&back).unwrap(), serde_json::to_value(&saved).unwrap());
    }
}
