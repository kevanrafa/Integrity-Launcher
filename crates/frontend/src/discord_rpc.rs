#![deny(unused_must_use)]

use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const DISCORD_APP_ID: u64 = 1473107584847188119;

pub struct DiscordPresence {
    enabled: Arc<AtomicBool>,
    instance_name: Arc<RwLock<Option<String>>>,
    start_time: Arc<RwLock<Option<i64>>>,
}

impl DiscordPresence {
    pub fn new() -> Self {
        log::info!("Discord RPC initializing with App ID: {}", DISCORD_APP_ID);

        Self {
            enabled: Arc::new(AtomicBool::new(true)),
            instance_name: Arc::new(RwLock::new(None)),
            start_time: Arc::new(RwLock::new(None)),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
        if !enabled {
            self.clear_presence();
        }
    }

    pub fn set_instance(&self, instance_name: &str) {
        *self.instance_name.write() = Some(instance_name.to_string());
        *self.start_time.write() =
            Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64);
        log::info!("Discord RPC: Set instance to {}", instance_name);
    }

    pub fn clear_instance(&self) {
        *self.instance_name.write() = None;
        *self.start_time.write() = None;
        self.clear_presence();
    }

    fn update_presence(&self) {
        if !self.is_enabled() {
            return;
        }

        let instance_name = self.instance_name.read().clone();
        let start_time = *self.start_time.read();

        if let Some(name) = instance_name {
            log::info!("Discord RPC presence set: Playing = {}, Started = {:?}", name, start_time);
        }
    }

    fn clear_presence(&self) {
        log::info!("Discord RPC presence cleared");
    }

    pub fn shutdown(&self) {
        self.clear_presence();
    }
}

impl Default for DiscordPresence {
    fn default() -> Self {
        Self::new()
    }
}
