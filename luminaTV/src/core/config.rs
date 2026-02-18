use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub resolution: ResolutionConfig,
    pub layers: Vec<LayerConfig>,
    pub decklink_monitor_enabled: bool,
    pub ndi_output_enabled: bool,
    pub decklink_output_enabled: bool,
    pub virtual_display_enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResolutionConfig {
    pub width: u32,
    pub height: u32,
    pub label: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LayerConfig {
    pub source_type: String, // "Camera", "Screen", "Presentation", "DeckLink", "None"
    pub source_name: String, // Display name or ID
    pub input_device_id: Option<String>, // Detailed ID if needed
    pub key_enabled: bool,
    #[serde(default)]
    pub key_params: KeyParamsConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyParamsConfig {
    pub color_u32: u32,
    pub tolerance: u32,
    pub softness: u32,
    pub luma_low: u32,
    pub luma_softness: u32,
    pub spill: bool,
    pub smoothing: u32,
}

impl Default for KeyParamsConfig {
    fn default() -> Self {
        Self {
            color_u32: 0x00B140, // TV Green default
            tolerance: 80,
            softness: 50,
            luma_low: 8,
            luma_softness: 10,
            spill: true,
            smoothing: 10,
        }
    }
}

pub struct ConfigManager;

impl ConfigManager {
    pub fn get_config_dir() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("LuminaTV");
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        path
    }

    pub fn save_preset(name: &str, config: &AppConfig) -> std::io::Result<()> {
        let mut path = Self::get_config_dir();
        path.push(format!("{}.json", name));
        let json = serde_json::to_string_pretty(config)?;
        fs::write(path, json)
    }

    pub fn load_preset(name: &str) -> Option<AppConfig> {
        let mut path = Self::get_config_dir();
        path.push(format!("{}.json", name));
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                return serde_json::from_str(&content).ok();
            }
        }
        None
    }

    pub fn save_last_session(config: &AppConfig) -> std::io::Result<()> {
        Self::save_preset("last_session", config)
    }

    pub fn load_last_session() -> Option<AppConfig> {
        Self::load_preset("last_session")
    }
    
    pub fn list_presets() -> Vec<String> {
        let path = Self::get_config_dir();
        let mut presets = Vec::new();
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "json" {
                        if let Some(stem) = entry.path().file_stem() {
                            presets.push(stem.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
        presets
    }
}
