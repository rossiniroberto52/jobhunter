use super::Source;
use crate::config::Config;
use crate::model::Job;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;

pub struct Remotive;

#[derive(Deserialize)]
struct Resp {
    jobs: Vec<RJob>,
}

#[derive(Deserialize)]
struct RJob {
    id: u64,
    url: String,
    title: String,
    company_name: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    candidate_required_location: String,
    #[serde(default)]
    description: String,
}

impl Source for Remotive {
    fn name(&self) -> &'static str {
        "remotive"
    }

    async fn fetch(&self, cfg: &Config) -> Result<Vec<Job>> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();

        let client = Client::new();

        // A API é pública e gratuita; evite rodar o fetch com muita frequência.
        for q in &cfg.search.queries {
            let resp: Resp = client
                .get("https://remotive.com/api/remote-jobs")
                .query(&[("category", "software-dev"), ("search", q), ("limit", "50")])
                .send()
                .await
                .with_context(|| format!("falha ao buscar '{q}' no Remotive"))?
                .json()
                .await
                .context("resposta do Remotive em formato inesperado")?;

            out.extend(convert(resp.jobs, &mut seen));
        }
        Ok(out)
    }
}

fn convert(jobs: Vec<RJob>, seen: &mut HashSet<u64>) -> Vec<Job> {
    let mut out = Vec::new();
    for j in jobs {
        if !seen.insert(j.id) {
            continue;
        }
        let desc: String = j.description.chars().take(3000).collect();
        out.push(Job {
            source: "remotive".to_string(),
            ext_id: j.id.to_string(),
            title: j.title,
            company: j.company_name,
            url: j.url,
            location: j.candidate_required_location,
            text: format!("{} {}", j.tags.join(" "), desc),
            score: 0,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parseia_resposta_de_exemplo() {
        let json = r#"{
            "job-count": 2,
            "jobs": [
              {"id": 1, "url": "https://remotive.com/a", "title": "Junior Java Developer",
               "company_name": "Acme", "category": "Software Development",
               "tags": ["java", "spring"], "job_type": "full_time",
               "candidate_required_location": "Brazil", "salary": "",
               "description": "<p>Spring Boot and SQL</p>"},
              {"id": 1, "url": "https://remotive.com/a", "title": "Dup",
               "company_name": "Acme"}
            ]}"#;
        let resp: Resp = serde_json::from_str(json).unwrap();
        let mut seen = HashSet::new();
        let jobs = convert(resp.jobs, &mut seen);
        assert_eq!(jobs.len(), 1); // o id repetido é descartado
        assert_eq!(jobs[0].location, "Brazil");
        assert!(jobs[0].text.contains("spring"));
    }
}
