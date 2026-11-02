use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub search: Search,
    pub weights: HashMap<String, i32>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub rss: Rss,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Rss {
    pub feeds: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Search {
    pub queries: Vec<String>,
    pub allowed_locations: Vec<String>,
}

pub fn load(path: &str) -> Result<Config> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("não achei o arquivo {path}"))?;
    toml::from_str(&raw).with_context(|| format!("config inválida em {path}"))
}
