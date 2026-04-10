use std::{collections::BTreeSet, sync::Arc};

use enumset::{EnumSet, EnumSetType};
use serde::{Deserialize, Serialize};
use crate::global_modpack::GlobalModpackProfile;

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct BackendConfig {
    #[serde(default, skip_serializing_if = "is_default_sync_targets", deserialize_with = "try_deserialize_sync_targets")]
    pub sync_targets: SyncTargets,
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub dont_open_game_output_when_launching: bool,
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub developer_mode: bool,
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub discord_rpc: DiscordRpcConfig,
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub proxy: ProxyConfig,
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub java_runtime: JavaRuntimeConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "crate::try_deserialize")]
    pub global_modpack_profiles: Vec<GlobalModpackProfile>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct JavaRuntimeConfig {
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub mode: JavaRuntimeMode,
}

impl Default for JavaRuntimeConfig {
    fn default() -> Self {
        Self {
            mode: JavaRuntimeMode::Auto,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum JavaRuntimeMode {
    #[default]
    Auto,
    System,
    Bundled,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DiscordRpcConfig {
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub enabled: bool,
    #[serde(default = "default_discord_rpc_client_id", skip_serializing_if = "String::is_empty", deserialize_with = "crate::try_deserialize")]
    pub client_id: String,
    #[serde(default = "default_idle_text", skip_serializing_if = "is_default_idle_text", deserialize_with = "crate::try_deserialize")]
    pub idle_text: String,
    #[serde(default = "default_selecting_text", skip_serializing_if = "is_default_selecting_text", deserialize_with = "crate::try_deserialize")]
    pub selecting_text: String,
    #[serde(default = "default_playing_text", skip_serializing_if = "is_default_playing_text", deserialize_with = "crate::try_deserialize")]
    pub playing_text: String,
    #[serde(default = "default_true", skip_serializing_if = "is_true", deserialize_with = "crate::try_deserialize")]
    pub show_advanced_details: bool,
}

impl Default for DiscordRpcConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            client_id: default_discord_rpc_client_id(),
            idle_text: "Integrity Launcher".to_string(),
            selecting_text: default_selecting_text(),
            playing_text: default_playing_text(),
            show_advanced_details: true,
        }
    }
}

fn default_discord_rpc_client_id() -> String {
    "1473107584847188119".to_string()
}

fn default_idle_text() -> String {
    "Integrity Launcher".to_string()
}

fn default_selecting_text() -> String {
    "Selecting Instance".to_string()
}

fn default_playing_text() -> String {
    "Playing Minecraft".to_string()
}

fn is_default_idle_text(value: &str) -> bool {
    value == default_idle_text()
}

fn is_default_selecting_text(value: &str) -> bool {
    value == default_selecting_text()
}

fn is_default_playing_text(value: &str) -> bool {
    value == default_playing_text()
}

fn default_true() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ProxyConfig {
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub protocol: ProxyProtocol,
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub host: String,
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub port: u16,
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub auth_enabled: bool,
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub username: String,
}

impl ProxyConfig {
    pub fn to_url(&self, password: Option<&str>) -> Option<String> {
        if !self.enabled || self.host.is_empty() {
            return None;
        }

        let scheme = self.protocol.scheme();

        if self.auth_enabled && !self.username.is_empty() {
            let password = password.unwrap_or("");
            // URL-encode username and password to handle special characters
            let username = urlencoding::encode(&self.username);
            let password = urlencoding::encode(password);
            Some(format!("{}://{}:{}@{}:{}", scheme, username, password, self.host, self.port))
        } else {
            Some(format!("{}://{}:{}", scheme, self.host, self.port))
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ProxyProtocol {
    #[default]
    Http,
    Https,
    Socks5,
}

impl ProxyProtocol {
    pub fn scheme(&self) -> &'static str {
        match self {
            ProxyProtocol::Http => "http",
            ProxyProtocol::Https => "https",
            ProxyProtocol::Socks5 => "socks5",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ProxyProtocol::Http => "HTTP",
            ProxyProtocol::Https => "HTTPS",
            ProxyProtocol::Socks5 => "SOCKS5",
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "HTTP" => ProxyProtocol::Http,
            "HTTPS" => ProxyProtocol::Https,
            "SOCKS5" => ProxyProtocol::Socks5,
            _ => ProxyProtocol::Http,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SyncTargets {
    pub files: BTreeSet<Arc<str>>,
    pub folders: BTreeSet<Arc<str>>,
}

fn is_default_sync_targets(sync_targets: &SyncTargets) -> bool {
    sync_targets.files.is_empty() && sync_targets.folders.is_empty()
}

fn try_deserialize_sync_targets<'de, D>(deserializer: D) -> Result<SyncTargets, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    // Migration from previous bitset
    if let serde_json::Value::Number(_) = value {
        let Ok(legacy) = EnumSet::<LegacySyncTarget>::deserialize(value) else {
            return Ok(SyncTargets::default());
        };

        let mut targets = SyncTargets::default();
        for legacy_target in legacy {
            let (name, file) = legacy_target.get_new_target();
            if file {
                targets.files.insert(name.into());
            } else {
                targets.folders.insert(name.into());
            }
        }
        return Ok(targets);
    }

    Ok(SyncTargets::deserialize(value).unwrap_or_default())
}

#[derive(Debug, enum_map::Enum, EnumSetType, strum::EnumIter)]
enum LegacySyncTarget {
    Options = 0,
    Servers = 1,
    Commands = 2,
    Hotbars = 13,
    Saves = 3,
    Config = 4,
    Screenshots = 5,
    Resourcepacks = 6,
    Shaderpacks = 7,
    Flashback = 8,
    DistantHorizons = 9,
    Voxy = 10,
    XaerosMinimap = 11,
    Bobby = 12,
    Litematic = 14,
}

impl LegacySyncTarget {
    pub fn get_new_target(self) -> (&'static str, bool) {
        match self {
            LegacySyncTarget::Options => ("options.txt", true),
            LegacySyncTarget::Servers => ("servers.dat", true),
            LegacySyncTarget::Commands => ("command_history.txt", true),
            LegacySyncTarget::Hotbars => ("hotbar.nbt", true),
            LegacySyncTarget::Saves => ("saves", false),
            LegacySyncTarget::Config => ("config", false),
            LegacySyncTarget::Screenshots => ("screenshots", false),
            LegacySyncTarget::Resourcepacks => ("resourcepacks", false),
            LegacySyncTarget::Shaderpacks => ("shaderpacks", false),
            LegacySyncTarget::Flashback => ("flashback", false),
            LegacySyncTarget::DistantHorizons => ("Distant_Horizons_server_data", false),
            LegacySyncTarget::Voxy => (".voxy", false),
            LegacySyncTarget::XaerosMinimap => ("xaero", false),
            LegacySyncTarget::Bobby => (".bobby", false),
            LegacySyncTarget::Litematic => ("schematics", false),
        }
    }
}
