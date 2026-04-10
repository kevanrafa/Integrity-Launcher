use std::time::{SystemTime, UNIX_EPOCH};

use discord_rich_presence::{
    activity::Activity,
    DiscordIpc,
    DiscordIpcClient,
};
use schema::backend_config::DiscordRpcConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RpcUiState {
    IdleInLauncher,
    SelectingInstance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayingState {
    pub instance_name: String,
    pub minecraft_version: String,
    pub loader: String,
    pub login_mode: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EffectivePresence {
    details: String,
    state: String,
}

pub struct DiscordRpcManager {
    config: DiscordRpcConfig,
    ui_state: RpcUiState,
    ui_selected_instance: Option<String>,
    playing_state: Option<PlayingState>,
    client: Option<DiscordIpcClient>,
    connected_client_id: Option<String>,
    last_presence: Option<EffectivePresence>,
    start_timestamp: Option<i64>,
}

impl DiscordRpcManager {
    pub fn new(config: DiscordRpcConfig) -> Self {
        Self {
            config,
            ui_state: RpcUiState::IdleInLauncher,
            ui_selected_instance: None,
            playing_state: None,
            client: None,
            connected_client_id: None,
            last_presence: None,
            start_timestamp: None,
        }
    }

    pub fn set_config(&mut self, config: DiscordRpcConfig) {
        self.config = config;
        self.reconcile();
    }

    pub fn set_ui_state(&mut self, state: RpcUiState, selected_instance: Option<String>) {
        self.ui_state = state;
        self.ui_selected_instance = selected_instance;
        if self.playing_state.is_none() {
            self.reconcile();
        }
    }

    pub fn set_playing_state(&mut self, playing: Option<PlayingState>) {
        self.playing_state = playing;
        if self.playing_state.is_some() {
            self.start_timestamp = Some(now_unix_ts());
        } else {
            self.start_timestamp = None;
        }
        self.reconcile();
    }

    pub fn shutdown(&mut self) {
        if let Some(client) = self.client.as_mut() {
            let _ = client.close();
        }
        self.client = None;
        self.connected_client_id = None;
        self.last_presence = None;
        self.start_timestamp = None;
    }

    fn reconcile(&mut self) {
        if !self.config.enabled || self.config.client_id.trim().is_empty() {
            self.shutdown();
            return;
        }

        if self.connected_client_id.as_deref() != Some(self.config.client_id.trim()) {
            self.shutdown();
        }

        if self.client.is_none() {
            let mut client = match DiscordIpcClient::new(self.config.client_id.trim()) {
                Ok(client) => client,
                Err(error) => {
                    log::debug!("Unable to create Discord IPC client: {error:?}");
                    return;
                }
            };

            if let Err(error) = client.connect() {
                log::debug!("Unable to connect Discord IPC client: {error:?}");
                return;
            }

            self.connected_client_id = Some(self.config.client_id.trim().to_string());
            self.client = Some(client);
            self.last_presence = None;
        }

        let presence = self.build_presence();
        if self.last_presence.as_ref() == Some(&presence) {
            return;
        }

        let mut activity = Activity::new()
            .details(presence.details.as_str())
            .state(presence.state.as_str());

        if self.playing_state.is_some() && self.config.show_advanced_details {
            if let Some(start_timestamp) = self.start_timestamp {
                activity = activity.timestamps(discord_rich_presence::activity::Timestamps::new().start(start_timestamp));
            }
        }

        let Some(client) = self.client.as_mut() else {
            return;
        };

        match client.set_activity(activity) {
            Ok(_) => {
                self.last_presence = Some(presence);
            }
            Err(error) => {
                log::debug!("Unable to set Discord RPC activity: {error:?}");
                self.shutdown();
            }
        }
    }

    fn build_presence(&self) -> EffectivePresence {
        if let Some(playing) = &self.playing_state {
            let state = self.config.playing_text.trim();
            let state = if state.is_empty() { "In Game" } else { state };
            let details = if self.config.show_advanced_details {
                format!(
                    "Playing: {} ({} {})",
                    playing.instance_name, playing.minecraft_version, playing.loader
                )
            } else {
                format!("Playing: {}", playing.instance_name)
            };
            return EffectivePresence {
                details,
                state: format!("{} | {}", playing.login_mode, state),
            };
        }

        match self.ui_state {
            RpcUiState::IdleInLauncher => {
                let details = self.config.idle_text.trim();
                let details = if details.is_empty() { "Integrity Launcher" } else { details };
                EffectivePresence {
                    details: details.to_string(),
                    state: "In Launcher".to_string(),
                }
            }
            RpcUiState::SelectingInstance => {
                let details = self.config.selecting_text.trim();
                let details = if details.is_empty() { "Selecting Instance" } else { details };
                let details = if self.config.show_advanced_details {
                    self.ui_selected_instance
                        .as_ref()
                        .map(|instance| format!("Selecting: {instance}"))
                        .unwrap_or_else(|| details.to_string())
                } else {
                    details.to_string()
                };
                EffectivePresence {
                    details,
                    state: "In Launcher".to_string(),
                }
            }
        }
    }
}

fn now_unix_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
