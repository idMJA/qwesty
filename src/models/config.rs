use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub discord: DiscordConfig,
    pub notifier: NotifierConfig,
    pub mode: Option<ModeConfig>,
    pub region: Option<RegionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub token: String,
    pub locale_mode: Option<String>,
    pub super_properties: String,
    pub webhooks: Option<Vec<WebhookEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEntry {
    pub name: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifierConfig {
    pub reward_filter: Option<String>,
    pub fetch_interval_minutes: Option<u64>,
    pub run_once: Option<bool>,
    pub storage_type: Option<String>,
    pub storage_path: Option<String>,
    pub initial_send_all: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    /// "collector" | "agent"
    pub role: Option<String>,
    /// Only for agents
    pub collector_url: Option<String>,
    /// Only for agents
    pub collector_token: Option<String>,
    /// Only for collector
    pub accept_token: Option<String>,
    /// Port for ingest server (collector only)
    pub ingest_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionConfig {
    /// Region code like "en-US", "ko-KR", etc.
    pub code: Option<String>,
}

impl Default for NotifierConfig {
    fn default() -> Self {
        Self {
            reward_filter: Some("all".to_string()),
            fetch_interval_minutes: Some(30),
            run_once: Some(false),
            storage_type: Some("json".to_string()),
            storage_path: Some("./known-quests.json".to_string()),
            initial_send_all: Some(false),
        }
    }
}

impl Config {
    /// Load configuration from `config.toml`.
    ///
    /// # Errors
    /// Returns `ConfigError` if file not found, IO fails, or parsing fails.
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from_toml("config.toml")
    }

    fn load_from_toml(path: &str) -> Result<Self, ConfigError> {
        if !Path::new(path).exists() {
            return Err(ConfigError::FileNotFound(path.to_string()));
        }

        let content = fs::read_to_string(path).map_err(|e| ConfigError::IoError(e.to_string()))?;

        let mut config: Self =
            toml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        if config.discord.token.is_empty() || config.discord.token == "your_discord_user_token" {
            return Err(ConfigError::InvalidToken(
                "Discord token is missing or not configured".to_string(),
            ));
        }

        // normalize role to lowercase
        if let Some(mode) = &mut config.mode {
            if let Some(role) = &mut mode.role {
                *role = role.trim().to_lowercase();
            }
        }

        Ok(config)
    }

    #[must_use]
    pub fn reward_filter(&self) -> &str {
        self.notifier.reward_filter.as_deref().unwrap_or("all")
    }

    #[must_use]
    pub fn fetch_interval(&self) -> u64 {
        self.notifier.fetch_interval_minutes.unwrap_or(30)
    }

    #[must_use]
    pub fn run_once(&self) -> bool {
        self.notifier.run_once.unwrap_or(false)
    }

    #[must_use]
    pub fn super_properties(&self) -> &str {
        if self.discord.super_properties.is_empty()
            || self.discord.super_properties == "your_base64_super_properties"
        {
            crate::utils::constants::DEFAULT_SUPER_PROPERTIES
        } else {
            &self.discord.super_properties
        }
    }

    #[must_use]
    pub fn storage_type(&self) -> &str {
        self.notifier.storage_type.as_deref().unwrap_or("json")
    }

    #[must_use]
    pub fn storage_path(&self) -> &str {
        self.notifier
            .storage_path
            .as_deref()
            .unwrap_or("./known-quests.json")
    }

    #[must_use]
    pub fn locale_mode(&self) -> &str {
        self.discord.locale_mode.as_deref().unwrap_or("single")
    }

    #[must_use]
    pub fn initial_send_all(&self) -> bool {
        self.notifier.initial_send_all.unwrap_or(false)
    }

    #[must_use]
    pub fn role(&self) -> &str {
        self.mode
            .as_ref()
            .and_then(|m| m.role.as_deref())
            .unwrap_or("collector")
    }

    #[must_use]
    pub fn is_collector(&self) -> bool {
        self.role() == "collector"
    }

    #[must_use]
    pub fn is_agent(&self) -> bool {
        self.role() == "agent"
    }

    #[must_use]
    pub fn collector_url(&self) -> Option<&str> {
        self.mode.as_ref().and_then(|m| m.collector_url.as_deref())
    }

    #[must_use]
    pub fn collector_token(&self) -> Option<&str> {
        self.mode
            .as_ref()
            .and_then(|m| m.collector_token.as_deref())
    }

    #[must_use]
    pub fn accept_token(&self) -> Option<&str> {
        self.mode.as_ref().and_then(|m| m.accept_token.as_deref())
    }

    #[must_use]
    pub fn region_code(&self) -> &str {
        self.region
            .as_ref()
            .and_then(|r| r.code.as_deref())
            .unwrap_or_else(|| self.locale_mode())
    }

    #[must_use]
    pub fn ingest_port(&self) -> u16 {
        self.mode
            .as_ref()
            .and_then(|m| m.ingest_port)
            .unwrap_or(8080)
    }
}

pub const LOCALES: &[&str] = &[
    "en-GB", "en-US", "da-DK", "de-DE", "nl-NL", "no-NO", "fi-FI", "sv-SE", "fr-FR", "it-IT",
    "es-ES", "es-419", "pt-BR", "hr-HR", "hu-HU", "lt-LT", "pl-PL", "ro-RO", "cs-CZ", "tr-TR",
    "el-GR", "bg-BG", "ru-RU", "uk-UA", "vi-VN", "hi-IN", "th-TH", "zh-CN", "zh-TW", "ja-JP",
    "ko-KR",
];

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Invalid token: {0}")]
    InvalidToken(String),
}
