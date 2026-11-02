use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;

/// Stop words comuns (PT + EN) que não indicam skill.
const STOP_WORDS: &[&str] = &[
    "a", "o", "os", "as", "um", "uma", "uns", "umas", "de", "do", "da", "dos", "das",
    "e", "ou", "mas", "por", "para", "com", "em", "no", "na", "nos", "nas", "sobre",
    "entre", "até", "mais", "menos", "muito", "pouco", "bem", "mal", "já", "não",
    "the", "and", "or", "but", "for", "with", "in", "on", "at", "to", "from", "by",
    "of", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
    "had", "do", "does", "did", "will", "would", "could", "should", "may", "might",
    "experiência", "experience", "anos", "years",
    "desenvolvedor", "developer", "engenheiro", "engineer", "analista", "analyst",
    "vaga", "job", "emprego", "position", "role", "cargo", "função", "function",
    "responsabilidades", "responsibilities", "requisitos", "requirements", "desejável",
    "preferível", "benefícios", "benefits", "salário", "salary", "remuneração",
    "contratação", "hiring", "processo", "process", "seleção", "selection",
];

/// Um perfil extraído do currículo.
#[derive(Debug, Clone, Default)]
pub struct Profile {
    /// Palavras-chave extraídas com contagem de ocorrências.
    pub keywords: HashMap<String, usize>,
    /// Nível sugerido com base em menções no currículo.
    pub level: Option<String>,
    /// Peso sugerido por palavra-chave (derivado da contagem).
    pub weights: HashMap<String, i32>,
}

impl Profile {
    /// Lê um arquivo de currículo (texto ou markdown) e extrai um perfil.
    pub fn from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("não consegui ler o currículo em {path}"))?;
        Ok(Self::from_text(&content))
    }

    /// Extrai palavras-chave de um texto bruto.
    pub fn from_text(text: &str) -> Self {
        let mut keywords: HashMap<String, usize> = HashMap::new();

        // Remove markup markdown simples e quebra em tokens.
        let clean = text
            .replace('#', " ")
            .replace('*', " ")
            .replace('_', " ")
            .replace('`', " ")
            .replace('-', " ")
            .replace('/', " ")
            .replace('\\', " ")
            .replace('(', " ")
            .replace(')', " ")
            .replace('[', " ")
            .replace(']', " ")
            .replace('|', " ")
            .replace(':', " ")
            .replace(';', " ")
            .replace(',', " ")
            .replace('.', " ")
            .replace('!', " ")
            .replace('?', " ")
            .replace('+', " ")
            .replace('=', " ")
            .replace('<', " ")
            .replace('>', " ")
            .replace('"', " ")
            .replace('\'', " ")
            .replace('\n', " ")
            .replace('\r', " ")
            .replace('\t', " ");

        for token in clean.split_whitespace() {
            let word = token.to_lowercase();
            if word.len() < 3 {
                continue;
            }
            if STOP_WORDS.iter().any(|&s| s == word) {
                continue;
            }
            // Mantém tokens alfanuméricos com hífen/ponto interno (ex: "spring-boot", "node.js").
            if word.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '.') {
                *keywords.entry(word).or_insert(0) += 1;
            }
        }

        let level = detect_level(&keywords);
        let weights = build_weights(&keywords);

        Self {
            keywords,
            level,
            weights,
        }
    }

    /// Top N palavras-chave por contagem.
    pub fn top_keywords(&self, n: usize) -> Vec<(String, usize)> {
        let mut v: Vec<(String, usize)> = self.keywords.iter().map(|(k, v)| (k.clone(), *v)).collect();
        v.sort_by(|a, b| b.cmp(a).then_with(|| a.0.cmp(&b.0)));
        v.truncate(n);
        v
    }
}

/// Detecta nível com base em menções no currículo.
fn detect_level(keywords: &HashMap<String, usize>) -> Option<String> {
    let senior = keywords.get("senior").or_else(|| keywords.get("sênior")).copied().unwrap_or(0);
    let pleno = keywords.get("pleno").copied().unwrap_or(0);
    let junior = keywords.get("junior").or_else(|| keywords.get("júnior")).copied().unwrap_or(0);
    let estagio = keywords.get("estagio").or_else(|| keywords.get("estágio")).copied().unwrap_or(0);
    let intern = keywords.get("intern").copied().unwrap_or(0);

    if senior > pleno && senior > junior && senior > estagio {
        Some("senior".to_string())
    } else if pleno > senior && pleno > junior {
        Some("pleno".to_string())
    } else if estagio > 0 || intern > 0 {
        Some("estagio".to_string())
    } else if junior > 0 {
        Some("junior".to_string())
    } else {
        None
    }
}

/// Converte contagens em pesos de 1 a 5 (escala absoluta, não relativa).
fn build_weights(keywords: &HashMap<String, usize>) -> HashMap<String, i32> {
    let mut weights = HashMap::new();
    for (word, count) in keywords {
        let weight = match *count {
            3..=4 => 4,
            2 => 3,
            1 => 1,
            _ if *count >= 5 => 5,
            _ => 0,
        };
        if weight > 0 {
            weights.insert(word.clone(), weight);
        }
    }
    weights
}

/// Salva os pesos sugeridos em um arquivo TOML separado (não sobrescreve config.toml inteiro).
pub fn save_weights_toml(profile: &Profile, path: &str) -> Result<()> {
    let mut lines = Vec::new();
    lines.push("# Pesos sugeridos a partir do currículo".to_string());
    lines.push("# Copie as que quiser para [weights] no config.toml".to_string());
    lines.push(String::new());

    let mut items: Vec<(&String, &i32)> = profile.weights.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    for (word, weight) in items {
        lines.push(format!("{word:?} = {weight}"));
    }

    fs::write(path, lines.join("\n")).with_context(|| format!("não consegui escrever {path}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrai_keywords_basicas() {
        let cv = "Desenvolvedor Java com Spring Boot e Python. Experiência com Docker, PostgreSQL e AWS.";
        let p = Profile::from_text(cv);
        assert!(p.keywords.contains_key("java"));
        assert!(p.keywords.contains_key("spring"));
        assert!(p.keywords.contains_key("boot"));
        assert!(p.keywords.contains_key("python"));
        assert!(p.keywords.contains_key("docker"));
        assert!(p.keywords.contains_key("postgresql"));
        assert!(p.keywords.contains_key("aws"));
    }

    #[test]
    fn remove_stop_words() {
        let cv = "o a os as e ou mas por para com em no na";
        let p = Profile::from_text(cv);
        assert!(p.keywords.is_empty());
    }

    #[test]
    fn detecta_senior() {
        let cv = "Senior Java Developer com 8 anos de experiência.";
        let p = Profile::from_text(cv);
        assert_eq!(p.level, Some("senior".to_string()));
    }

    #[test]
    fn detecta_estagio() {
        let cv = "Estagiário de desenvolvimento estagio intern.";
        let p = Profile::from_text(cv);
        assert_eq!(p.level, Some("estagio".to_string()));
    }
}
