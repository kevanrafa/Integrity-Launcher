use std::sync::Arc;

use auth::models::MinecraftAccessToken;
use bridge::{account::Account, message::MessageToFrontend};
use rustc_hash::FxHashMap;
use schema::{minecraft_profile::MinecraftProfileResponse, unique_bytes::UniqueBytes};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct MinecraftLoginInfo {
    pub uuid: Uuid,
    pub username: Arc<str>,
    pub access_token: MinecraftAccessToken,
    pub offline: bool,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct BackendAccountInfo {
    pub accounts: FxHashMap<Uuid, BackendAccount>,
    pub selected_account: Option<Uuid>,
}

impl BackendAccountInfo {
    pub fn create_update_message(&self) -> MessageToFrontend {
        let mut accounts = Vec::with_capacity(self.accounts.len());
        for (uuid, account) in &self.accounts {
            accounts.push(Account {
                uuid: *uuid,
                username: account.username.clone(),
                offline: account.is_offline(),
                head: account.head.clone(),
            });
        }
        accounts.sort_by(|a, b| lexical_sort::natural_lexical_cmp(&a.username, &b.username));
        MessageToFrontend::AccountsUpdated {
            accounts: accounts.into(),
            selected_account: self.selected_account,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendAccountType {
    #[default]
    Microsoft,
    Offline,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackendAccount {
    pub username: Arc<str>,
    #[serde(default)]
    pub account_type: BackendAccountType,
    #[serde(default)]
    pub offline: bool,
    pub head: Option<UniqueBytes>,
}

impl BackendAccount {
    pub fn new_from_profile(profile: &MinecraftProfileResponse) -> Self {
        Self {
            username: profile.name.clone(),
            account_type: BackendAccountType::Microsoft,
            offline: false,
            head: None,
        }
    }

    pub fn new_offline(username: Arc<str>) -> Self {
        Self {
            username,
            account_type: BackendAccountType::Offline,
            offline: true,
            head: None,
        }
    }

    pub fn is_offline(&self) -> bool {
        self.account_type == BackendAccountType::Offline || self.offline
    }
}
