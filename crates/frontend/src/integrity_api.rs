use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context as _};
use futures::AsyncReadExt;
use gpui::http_client::HttpClient;
use serde::Deserialize;

const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub const NEWS_URL: &str = "https://kevanrafa.github.io/Integrity-Modpack/api/news.json";
pub const MODPACKS_URL: &str = "https://kevanrafa.github.io/Integrity-Modpack/api/modpacks.json";
pub const FEATURED_URL: &str = "https://kevanrafa.github.io/Integrity-Modpack/api/featured.json";
pub const LAUNCHER_URL: &str = "https://kevanrafa.github.io/Integrity-Modpack/api/launcher.json";

static RESPONSE_CACHE: LazyLock<Mutex<HashMap<&'static str, CacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

struct CacheEntry {
    fetched_at: Instant,
    body: Arc<str>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct IntegrityModpack {
    #[serde(default, alias = "slug", alias = "project_id")]
    pub id: Option<Arc<str>>,
    #[serde(default, alias = "title")]
    pub name: Option<Arc<str>>,
    #[serde(default, alias = "latest_version", alias = "version_number")]
    pub version: Option<Arc<str>>,
    #[serde(default, alias = "summary")]
    pub description: Option<Arc<str>>,
    #[serde(default, alias = "icon", alias = "icon_url", alias = "image")]
    pub icon_url: Option<Arc<str>>,
    #[serde(default, alias = "download", alias = "downloadUrl", alias = "download_url", alias = "mrpack", alias = "mrpack_url", alias = "url")]
    pub download_url: Option<Arc<str>>,
    #[serde(default)]
    pub loader: Option<Arc<str>>,
    #[serde(default, alias = "game_version", alias = "mc_version", alias = "minecraft")]
    pub minecraft_version: Option<Arc<str>>,
    #[serde(default, alias = "changelogUrl", alias = "changelog_url")]
    pub changelog: Option<Arc<str>>,
}

impl IntegrityModpack {
    pub fn display_name(&self) -> Arc<str> {
        self.name.clone().unwrap_or_else(|| Arc::from("Unnamed Modpack"))
    }

    pub fn display_version(&self) -> Arc<str> {
        self.version.clone().unwrap_or_else(|| Arc::from("Unknown version"))
    }

    pub fn loader_line(&self) -> Arc<str> {
        match (&self.loader, &self.minecraft_version) {
            (Some(loader), Some(version)) => Arc::from(format!("{loader} {version}")),
            (Some(loader), None) => loader.clone(),
            (None, Some(version)) => Arc::from(format!("Minecraft {version}")),
            (None, None) => Arc::from("Modpack"),
        }
    }

    fn matches_feature_key(&self, key: &str) -> bool {
        self.id.as_deref() == Some(key) || self.name.as_deref() == Some(key)
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct IntegrityNewsItem {
    #[serde(default)]
    pub title: Option<Arc<str>>,
    #[serde(default, alias = "body", alias = "description")]
    pub content: Option<Arc<str>>,
    #[serde(default, alias = "date", alias = "created_at", alias = "published_at")]
    pub timestamp: Option<Arc<str>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct IntegrityLauncherInfo {
    #[serde(default, alias = "version")]
    pub latest: Option<Arc<str>>,
    #[serde(default, alias = "release", alias = "release_url", alias = "download_url", alias = "update_url")]
    pub release_url: Option<Arc<str>>,
    #[serde(default)]
    pub maintenance: bool,
    #[serde(default, alias = "message", alias = "maintenanceMessage")]
    pub maintenance_message: Option<Arc<str>>,
}

#[derive(Clone, Debug, Default)]
pub struct IntegrityModpackCatalog {
    pub featured: Vec<IntegrityModpack>,
    pub modpacks: Vec<IntegrityModpack>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ListEnvelope<T> {
    List(Vec<T>),
    Object {
        #[serde(default)]
        news: Vec<T>,
        #[serde(default)]
        items: Vec<T>,
        #[serde(default)]
        modpacks: Vec<T>,
        #[serde(default)]
        featured: Vec<T>,
    },
}

impl<T> ListEnvelope<T> {
    fn into_vec(self) -> Vec<T> {
        match self {
            ListEnvelope::List(items) => items,
            ListEnvelope::Object {
                news,
                items,
                modpacks,
                featured,
            } => news
                .into_iter()
                .chain(items)
                .chain(modpacks)
                .chain(featured)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum FeaturedEntry {
    Key(Arc<str>),
    Modpack(IntegrityModpack),
}

pub async fn load_modpack_catalog(
    client: Arc<dyn HttpClient>,
) -> anyhow::Result<IntegrityModpackCatalog> {
    let modpacks: Vec<IntegrityModpack> = fetch_list(client.clone(), MODPACKS_URL).await?;
    let featured_entries: Vec<FeaturedEntry> = fetch_list(client, FEATURED_URL).await?;

    let featured = featured_entries
        .into_iter()
        .filter_map(|entry| match entry {
            FeaturedEntry::Modpack(modpack) => Some(modpack),
            FeaturedEntry::Key(key) => modpacks
                .iter()
                .find(|modpack| modpack.matches_feature_key(&key))
                .cloned(),
        })
        .collect();

    Ok(IntegrityModpackCatalog { featured, modpacks })
}

pub async fn load_news(client: Arc<dyn HttpClient>) -> anyhow::Result<Vec<IntegrityNewsItem>> {
    let mut news: Vec<IntegrityNewsItem> = fetch_list(client, NEWS_URL).await?;
    news.sort_by(|a, b| news_timestamp(b).cmp(&news_timestamp(a)));
    Ok(news)
}

pub async fn load_launcher_info(
    client: Arc<dyn HttpClient>,
) -> anyhow::Result<IntegrityLauncherInfo> {
    fetch_json(client, LAUNCHER_URL).await
}

pub fn current_launcher_version() -> &'static str {
    option_env!("INTEGRITY_LAUNCHER_VERSION")
        .or(option_env!("PANDORA_RELEASE_VERSION"))
        .unwrap_or("0.0.0-dev")
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest = version_parts(latest);
    let current = version_parts(current);
    latest > current
}

async fn fetch_list<T>(client: Arc<dyn HttpClient>, url: &'static str) -> anyhow::Result<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    let envelope: ListEnvelope<T> = fetch_json(client, url).await?;
    Ok(envelope.into_vec())
}

async fn fetch_json<T>(client: Arc<dyn HttpClient>, url: &'static str) -> anyhow::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let body = fetch_text(client, url).await?;
    serde_json::from_str(&body).with_context(|| format!("Invalid JSON from {url}"))
}

async fn fetch_text(client: Arc<dyn HttpClient>, url: &'static str) -> anyhow::Result<Arc<str>> {
    if let Some(body) = cached_body(url) {
        return Ok(body);
    }

    let mut response = tokio::time::timeout(REQUEST_TIMEOUT, client.get(url, ().into(), true))
        .await
        .map_err(|_| anyhow!("Request timed out after 10 seconds: {url}"))?
        .map_err(|error| anyhow!(error))?;

    if !response.status().is_success() {
        return Err(anyhow!("Request failed with status {}: {url}", response.status()));
    }

    let mut body = String::new();
    response
        .body_mut()
        .read_to_string(&mut body)
        .await
        .with_context(|| format!("Unable to read response from {url}"))?;

    let body: Arc<str> = body.into();
    RESPONSE_CACHE.lock().unwrap().insert(
        url,
        CacheEntry {
            fetched_at: Instant::now(),
            body: body.clone(),
        },
    );
    Ok(body)
}

fn cached_body(url: &'static str) -> Option<Arc<str>> {
    let cache = RESPONSE_CACHE.lock().unwrap();
    let entry = cache.get(url)?;
    (entry.fetched_at.elapsed() < CACHE_TTL).then(|| entry.body.clone())
}

fn news_timestamp(item: &IntegrityNewsItem) -> i64 {
    let Some(timestamp) = item.timestamp.as_deref() else {
        return 0;
    };
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|date| date.timestamp())
        .unwrap_or(0)
}

fn version_parts(version: &str) -> Vec<u32> {
    version
        .trim_start_matches('v')
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u32>().ok())
        .collect()
}
