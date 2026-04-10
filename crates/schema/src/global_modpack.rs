use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobalModpackProfile {
    #[serde(default, skip_serializing_if = "String::is_empty", deserialize_with = "crate::try_deserialize")]
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty", deserialize_with = "crate::try_deserialize")]
    pub display_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty", deserialize_with = "crate::try_deserialize")]
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "crate::try_deserialize")]
    pub source_url: Option<Arc<str>>,
    #[serde(default = "crate::default_true", skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "crate::try_deserialize")]
    pub global_overrides: Vec<Arc<str>>,
}
