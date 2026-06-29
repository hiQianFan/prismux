use crate::{
    compatibility::{CONTROL_PLANE_SCHEMA_VERSION, STATE_SCHEMA_VERSION},
    settings::{SETTINGS_SCHEMA_VERSION, settings_storage_path},
};
use omx_core::{Result, storage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AboutView {
    pub schema_version: u32,
    pub app_version: String,
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub settings_schema_version: u32,
    pub runtime: AboutRuntime,
    pub state_root: AboutPath,
    pub settings_path: AboutPath,
    pub links: Vec<AboutLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AboutRuntime {
    pub mode: String,
    pub status_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AboutPath {
    pub display: String,
    pub reveal_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AboutLink {
    pub label: String,
    pub url: String,
}

pub fn about_view() -> Result<AboutView> {
    let state_root = storage::state_root()?;
    let settings_path = settings_storage_path()?;
    Ok(AboutView {
        schema_version: 1,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        control_plane_schema_version: CONTROL_PLANE_SCHEMA_VERSION,
        state_schema_version: STATE_SCHEMA_VERSION,
        settings_schema_version: SETTINGS_SCHEMA_VERSION,
        runtime: AboutRuntime {
            mode: "embedded_staticlib".to_string(),
            status_text: "Menubar backend embedded through Rust static library".to_string(),
        },
        state_root: AboutPath {
            display: storage::display_path(&state_root),
            reveal_path: Some(storage::display_path(&state_root)),
        },
        settings_path: AboutPath {
            display: storage::display_path(&settings_path),
            reveal_path: settings_path.parent().map(storage::display_path),
        },
        links: vec![
            AboutLink {
                label: "Repository".to_string(),
                url: "https://github.com/Sitoi/OpenMux".to_string(),
            },
            AboutLink {
                label: "Documentation".to_string(),
                url: "https://github.com/Sitoi/OpenMux/tree/main/docs".to_string(),
            },
        ],
    })
}
