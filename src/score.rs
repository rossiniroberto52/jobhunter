use crate::config::Config;
use crate::model::Job;

/// Procura `kw` em `hay` respeitando limites de palavra,
/// assim "java" não casa com "javascript".
fn contains_word(hay: &str, kw: &str) -> bool {
    if kw.is_empty() {
        return false;
    }
    let mut start = 0;
    while let Some(pos) = hay[start..].find(kw) {
        let i = start + pos;
        let j = i + kw.len();
        let before_ok = hay[..i].chars().next_back().map_or(true, |c| !c.is_alphanumeric());
        let after_ok = hay[j..].chars().next().map_or(true, |c| !c.is_alphanumeric());
        if before_ok && after_ok {
            return true;
        }
        start = j;
    }
    false
}

pub fn location_ok(job: &Job, cfg: &Config) -> bool {
    let loc = job.location.to_lowercase();
    loc.is_empty()
        || cfg
            .search
            .allowed_locations
            .iter()
            .any(|a| loc.contains(&a.to_lowercase()))
}

pub fn score(job: &Job, cfg: &Config) -> i32 {
    let hay = format!("{} {}", job.title, job.text).to_lowercase();
    let mut s = 0;
    for (kw, w) in &cfg.weights {
        if contains_word(&hay, &kw.to_lowercase()) {
            s += w;
        }
    }
    for kw in &cfg.exclude {
        if contains_word(&hay, &kw.to_lowercase()) {
            s -= 5;
        }
    }
    s.clamp(0, 10)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Rss, Search};
    use std::collections::HashMap;

    fn cfg() -> Config {
        let mut weights = HashMap::new();
        weights.insert("java".to_string(), 3);
        weights.insert("junior".to_string(), 2);
        Config {
            search: Search {
                queries: vec![],
                allowed_locations: vec!["brazil".into(), "worldwide".into()],
            },
            weights,
            exclude: vec!["senior".into()],
            rss: Rss { feeds: vec![] },
        }
    }

    fn job(title: &str, text: &str, loc: &str) -> Job {
        Job {
            source: "t".into(),
            ext_id: "1".into(),
            title: title.into(),
            company: "c".into(),
            url: "u".into(),
            location: loc.into(),
            text: text.into(),
            score: 0,
        }
    }

    #[test]
    fn java_nao_casa_com_javascript() {
        assert!(!contains_word("javascript developer", "java"));
        assert!(contains_word("java developer", "java"));
        assert!(contains_word("(java)", "java"));
    }

    #[test]
    fn pontua_e_penaliza() {
        let c = cfg();
        assert_eq!(score(&job("Junior Java Developer", "", "Brazil"), &c), 5);
        assert_eq!(score(&job("Senior Java Developer", "", "Brazil"), &c), 0);
    }

    #[test]
    fn filtra_localizacao() {
        let c = cfg();
        assert!(location_ok(&job("x", "", "Worldwide"), &c));
        assert!(!location_ok(&job("x", "", "USA Only"), &c));
    }
}
