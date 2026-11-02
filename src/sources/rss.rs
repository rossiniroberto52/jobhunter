use super::Source;
use crate::config::Config;
use crate::model::Job;
use anyhow::{Context, Result};
use feed_rs::parser;
use reqwest::Client;
use std::collections::HashSet;

pub struct Rss;

impl Source for Rss {
    fn name(&self) -> &'static str {
        "rss"
    }

    async fn fetch(&self, cfg: &Config) -> Result<Vec<Job>> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();

        for url in &cfg.rss.feeds {
            let client = Client::new();
            let text = client
                .get(url)
                .send()
                .await
                .with_context(|| format!("falha ao baixar RSS {url}"))?
                .text()
                .await
                .with_context(|| format!("resposta inválida do RSS {url}"))?;

            let feed = parser::parse(text.as_bytes())
                .with_context(|| format!("falha ao fazer parse do RSS {url}"))?;

            let company = feed
                .title
                .as_ref()
                .map(|t| t.content.clone())
                .unwrap_or_else(|| "RSS".to_string());

            for entry in feed.entries {
                let id = if entry.id.is_empty() {
                    url.clone()
                } else {
                    entry.id.clone()
                };
                if !seen.insert(id.clone()) {
                    continue;
                }
                let title = entry
                    .title
                    .as_ref()
                    .map(|t| t.content.clone())
                    .unwrap_or_else(|| "(sem título)".to_string());
                let url_str = entry
                    .links
                    .first()
                    .map(|l| l.href.clone())
                    .unwrap_or_else(|| url.clone());
                let content = entry
                    .content
                    .as_ref()
                    .and_then(|c| c.body.clone())
                    .unwrap_or_default();
                let summary = entry
                    .summary
                    .as_ref()
                    .map(|t| t.content.clone())
                    .unwrap_or_default();
                let text = format!("{} {} {}", title, content, summary);

                out.push(Job {
                    source: "rss".to_string(),
                    ext_id: id,
                    title,
                    company: company.clone(),
                    url: url_str,
                    location: String::new(),
                    text,
                    score: 0,
                });
            }
        }
        Ok(out)
    }
}
