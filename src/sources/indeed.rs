use super::Source;
use crate::config::Config;
use crate::model::Job;
use anyhow::{Context, Result};
use feed_rs::parser;
use std::collections::HashSet;

pub struct Indeed;

impl Source for Indeed {
    fn name(&self) -> &'static str {
        "indeed"
    }

    async fn fetch(&self, cfg: &Config) -> Result<Vec<Job>> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();

        for q in &cfg.search.queries {
            // Indeed RSS: https://www.indeed.com.br/rss?q=TERMO&l=LOCAL&sort=date
            let url = format!(
                "https://www.indeed.com.br/rss?q={}&sort=date&fromage=7",
                urlencoding::encode(q)
            );

            let text = reqwest::Client::new()
                .get(&url)
                .header("User-Agent", "Mozilla/5.0 (compatible; jobhunter/0.1)")
                .send()
                .await
                .with_context(|| format!("falha ao baixar Indeed RSS: {url}"))?
                .text()
                .await
                .with_context(|| format!("resposta inválida do Indeed: {url}"))?;

            let feed = parser::parse(text.as_bytes())
                .with_context(|| format!("falha ao fazer parse do Indeed RSS: {url}"))?;

            let company = feed
                .title
                .as_ref()
                .map(|t| t.content.clone())
                .unwrap_or_else(|| "Indeed".to_string());

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

                // Tenta extrair localização do summary ou content
                let location = extrai_localizacao(&text);

                out.push(Job {
                    source: "indeed".to_string(),
                    ext_id: id,
                    title,
                    company: company.clone(),
                    url: url_str,
                    location,
                    text,
                    score: 0,
                });
            }
        }
        Ok(out)
    }
}

/// Tenta extrair localização do texto (cidade/estado/país comuns),
/// respeitando limites de palavra pra não casar "office" com "ce".
fn extrai_localizacao(text: &str) -> String {
    let lugares = [
        "brasil", "brazil", "são paulo", "sao paulo", "sp", "rio de janeiro",
        "rj", "belo horizonte", "mg", "curitiba", "pr", "porto alegre", "rs",
        "salvador", "ba", "recife", "pe", "fortaleza", "ce", "brasília",
        "brasilia", "df", "remote", "remoto", "home office", "hybrido",
        "híbrido", "presencial",
    ];
    let lower = text.to_lowercase();
    for lugar in lugares {
        if palavra_contida(&lower, lugar) {
            return lugar.to_string();
        }
    }
    String::new()
}

/// Verifica se `agulha` aparece em `palheiro` como palavra inteira
/// (não colada a outras letras), assim "office" não casa com "ce".
fn palavra_contida(palheiro: &str, agulha: &str) -> bool {
    if agulha.is_empty() {
        return false;
    }
    let mut start = 0;
    while let Some(pos) = palheiro[start..].find(agulha) {
        let i = start + pos;
        let j = i + agulha.len();
        let before_ok = i == 0 || !palheiro[..i].chars().next_back().unwrap().is_alphanumeric();
        let after_ok = j == palheiro.len() || !palheiro[j..].chars().next().unwrap().is_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        start = j;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrai_localizacao_basica() {
        assert_eq!(extrai_localizacao("Vaga em São Paulo, SP"), "são paulo");
        assert_eq!(extrai_localizacao("Home office remoto"), "remoto");
        assert_eq!(extrai_localizacao("Vaga em Curitiba - PR"), "curitiba");
        assert_eq!(extrai_localizacao("Vaga em Marte"), "");
    }

    #[test]
    fn nao_casa_office_com_ce() {
        assert_eq!(extrai_localizacao("Trabalho em office"), "");
    }
}
