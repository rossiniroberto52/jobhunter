use super::Source;
use crate::config::Config;
use crate::model::Job;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;

pub struct Wellfound;

#[derive(Deserialize)]
struct Resp {
    data: Vec<WFJob>,
    #[allow(dead_code)]
    next_page: Option<String>,
}

#[derive(Deserialize)]
struct WFJob {
    id: u64,
    title: String,
    company_name: String,
    url: String,
    location: String,
    description: String,
    #[serde(default)]
    tags: Vec<String>,
}

impl Source for Wellfound {
    fn name(&self) -> &'static str {
        "wellfound"
    }

    async fn fetch(&self, cfg: &Config) -> Result<Vec<Job>> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();

        let client = Client::new();

        for q in &cfg.search.queries {
            let resp: Resp = client
                .get("https://api.wellfound.com/v1/jobs")
                .query(&[
                    ("filter[keyword]", q.as_str()),
                    ("filter[remote]", "true"),
                    ("limit", "50"),
                ])
                .send()
                .await
                .with_context(|| format!("falha ao buscar '{q}' no Wellfound"))?
                .json()
                .await
                .context("resposta do Wellfound em formato inesperado")?;

            out.extend(convert(resp.data, &mut seen));
        }
        Ok(out)
    }
}

fn convert(jobs: Vec<WFJob>, seen: &mut HashSet<u64>) -> Vec<Job> {
    let mut out = Vec::new();
    for j in jobs {
        if !seen.insert(j.id) {
            continue;
        }
        let text = format!("{} {} {}", j.tags.join(" "), j.title, j.description);
        out.push(Job {
            source: "wellfound".to_string(),
            ext_id: j.id.to_string(),
            title: j.title,
            company: j.company_name,
            url: j.url,
            location: j.location,
            text,
            score: 0,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_by_id() {
        let jobs = vec![
            WFJob { id: 1, title: "A".into(), company_name: "C".into(), url: "u".into(), location: "BR".into(), description: "".into(), tags: vec![] },
            WFJob { id: 1, title: "Dup".into(), company_name: "C".into(), url: "u".into(), location: "BR".into(), description: "".into(), tags: vec![] },
        ];
        let mut seen = HashSet::new();
        let out = convert(jobs, &mut seen);
        assert_eq!(out.len(), 1);
    }
}
