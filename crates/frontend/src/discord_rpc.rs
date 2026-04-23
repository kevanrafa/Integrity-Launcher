#![deny(unused_must_use)]

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread;

use gpui::{App, Global};
use parking_lot::RwLock;
use rand::{Rng, seq::SliceRandom};
use serde_json::json;

const DISCORD_APP_ID: &str = "1473107584847188119";
const IPC_HANDSHAKE: u32 = 0;
const IPC_FRAME: u32 = 1;
const IDLE_LINES: &[&str] = &[
    "Preparing the next integrity check",
    "Polishing launch parameters",
    "Guarding modded worlds",
    "Reviewing launcher systems",
];

const INSTANCE_LINES: &[&str] = &[
    "Holding the line",
    "Calibrating ancient machinery",
    "Patching reality one tick at a time",
    "Staring down crash logs",
    "Making legacy behave",
    "Building something stubborn",
];

#[derive(Clone)]
pub struct DiscordPresence {
    enabled: Arc<AtomicBool>,
    sender: mpsc::Sender<RpcCommand>,
    instance_name: Arc<RwLock<Option<String>>>,
}

struct DiscordPresenceGlobal {
    presence: DiscordPresence,
}

impl Global for DiscordPresenceGlobal {}

enum RpcCommand {
    SetEnabled(bool),
    SetInstance(Option<String>),
    Shutdown,
}

enum DiscordIpcConnection {
    File(std::fs::File),
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixStream),
}

impl Read for DiscordIpcConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::File(file) => file.read(buf),
            #[cfg(unix)]
            Self::Unix(stream) => stream.read(buf),
        }
    }
}

impl Write for DiscordIpcConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::File(file) => file.write(buf),
            #[cfg(unix)]
            Self::Unix(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::File(file) => file.flush(),
            #[cfg(unix)]
            Self::Unix(stream) => stream.flush(),
        }
    }
}

impl DiscordPresence {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let enabled = Arc::new(AtomicBool::new(true));
        let instance_name = Arc::new(RwLock::new(None));

        thread::spawn(move || rpc_worker(receiver));

        Self {
            enabled,
            sender,
            instance_name,
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
        let _ = self.sender.send(RpcCommand::SetEnabled(enabled));
    }

    pub fn set_instance(&self, instance_name: &str) {
        *self.instance_name.write() = Some(instance_name.to_string());
        let _ = self.sender.send(RpcCommand::SetInstance(Some(instance_name.to_string())));
    }

    pub fn clear_instance(&self) {
        *self.instance_name.write() = None;
        let _ = self.sender.send(RpcCommand::SetInstance(None));
    }

    pub fn shutdown(&self) {
        let _ = self.sender.send(RpcCommand::Shutdown);
    }
}

impl Default for DiscordPresence {
    fn default() -> Self {
        Self::new()
    }
}

pub fn init(cx: &mut App) {
    cx.set_global(DiscordPresenceGlobal {
        presence: DiscordPresence::new(),
    });
}

pub fn global(cx: &App) -> DiscordPresence {
    cx.global::<DiscordPresenceGlobal>().presence.clone()
}

pub fn sync_enabled_from_config(enabled: bool, cx: &mut App) {
    global(cx).set_enabled(enabled);
}

pub fn set_running_instance(instance_name: &str, cx: &mut App) {
    global(cx).set_instance(instance_name);
}

pub fn clear_running_instance(cx: &mut App) {
    global(cx).clear_instance();
}

pub fn shutdown_global(cx: &mut App) {
    global(cx).shutdown();
}

fn rpc_worker(receiver: mpsc::Receiver<RpcCommand>) {
    let mut connection: Option<DiscordIpcConnection> = None;
    let mut enabled = true;
    let mut instance_name: Option<String> = None;

    while let Ok(command) = receiver.recv() {
        match command {
            RpcCommand::SetEnabled(value) => {
                enabled = value;
                if enabled {
                    update_presence(&mut connection, instance_name.as_deref());
                } else {
                    clear_presence(&mut connection);
                }
            }
            RpcCommand::SetInstance(new_instance) => {
                instance_name = new_instance;
                if enabled {
                    update_presence(&mut connection, instance_name.as_deref());
                }
            }
            RpcCommand::Shutdown => {
                clear_presence(&mut connection);
                break;
            }
        }
    }
}

fn update_presence(connection: &mut Option<DiscordIpcConnection>, instance_name: Option<&str>) {
    let Some(active_connection) = ensure_connected(connection) else {
        return;
    };

    let activity = match instance_name {
        Some(instance_name) => json!({
            "details": format!("Di Instance {instance_name}"),
            "state": random_line(INSTANCE_LINES),
            "timestamps": {
                "start": current_timestamp(),
            },
        }),
        None => json!({
            "details": "Inside Integrity Launcher",
            "state": random_line(IDLE_LINES),
        }),
    };

    let payload = json!({
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": std::process::id(),
            "activity": activity,
        },
        "nonce": nonce(),
    });

    let reconnect_needed = if let Err(err) = send_frame(active_connection, IPC_FRAME, &payload.to_string()) {
        log::debug!("Discord RPC set activity failed: {err}");
        true
    } else {
        false
    };

    if reconnect_needed {
        *connection = None;
    }
}

fn clear_presence(connection: &mut Option<DiscordIpcConnection>) {
    let Some(active_connection) = ensure_connected(connection) else {
        return;
    };

    let payload = json!({
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": std::process::id(),
            "activity": serde_json::Value::Null,
        },
        "nonce": nonce(),
    });

    let reconnect_needed = if let Err(err) = send_frame(active_connection, IPC_FRAME, &payload.to_string()) {
        log::debug!("Discord RPC clear activity failed: {err}");
        true
    } else {
        false
    };

    if reconnect_needed {
        *connection = None;
    }
}

fn ensure_connected(connection: &mut Option<DiscordIpcConnection>) -> Option<&mut DiscordIpcConnection> {
    if connection.is_none() {
        *connection = reconnect();
    }
    connection.as_mut()
}

fn reconnect() -> Option<DiscordIpcConnection> {
    let mut connection = connect()?;
    let payload = json!({
        "v": 1,
        "client_id": DISCORD_APP_ID,
    });

    if let Err(err) = send_frame(&mut connection, IPC_HANDSHAKE, &payload.to_string()) {
        log::debug!("Discord RPC handshake failed: {err}");
        return None;
    }

    let mut header = [0_u8; 8];
    if let Err(err) = connection.read_exact(&mut header) {
        log::debug!("Discord RPC handshake response failed: {err}");
        return None;
    }

    let payload_len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
    let mut payload = vec![0_u8; payload_len];
    if let Err(err) = connection.read_exact(&mut payload) {
        log::debug!("Discord RPC payload read failed: {err}");
        return None;
    }

    Some(connection)
}

fn connect() -> Option<DiscordIpcConnection> {
    for path in candidate_paths() {
        #[cfg(unix)]
        if let Ok(stream) = std::os::unix::net::UnixStream::connect(&path) {
            return Some(DiscordIpcConnection::Unix(stream));
        }

        if let Ok(file) = OpenOptions::new().read(true).write(true).open(&path) {
            return Some(DiscordIpcConnection::File(file));
        }
    }

    None
}

fn candidate_paths() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        (0..10)
            .map(|index| PathBuf::from(format!(r"\\.\pipe\discord-ipc-{index}")))
            .collect()
    }

    #[cfg(unix)]
    {
        let mut roots = Vec::new();
        if let Some(path) = std::env::var_os("XDG_RUNTIME_DIR") {
            roots.push(PathBuf::from(path));
        }
        if let Some(home) = std::env::var_os("HOME") {
            roots.push(Path::new(&home).join(".config/discord"));
        }
        roots.push(PathBuf::from("/tmp"));

        let mut paths = Vec::new();
        for root in roots {
            for index in 0..10 {
                paths.push(root.join(format!("discord-ipc-{index}")));
            }
        }
        paths
    }
}

fn send_frame(connection: &mut DiscordIpcConnection, opcode: u32, payload: &str) -> std::io::Result<()> {
    let payload = payload.as_bytes();
    connection.write_all(&opcode.to_le_bytes())?;
    connection.write_all(&(payload.len() as u32).to_le_bytes())?;
    connection.write_all(payload)?;
    connection.flush()?;
    Ok(())
}

fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn nonce() -> String {
    format!(
        "{}-{}",
        current_timestamp(),
        rand::thread_rng().gen_range(10_000_u64..99_999_u64)
    )
}

fn random_line(lines: &[&str]) -> String {
    let mut rng = rand::thread_rng();
    if rng.gen_ratio(1, 24) {
        "ROOT access acknowledged".to_string()
    } else {
        lines.choose(&mut rng).copied().unwrap_or("Integrity holds").to_string()
    }
}
