pub mod remotive;
pub mod wellfound;
pub mod rss;
pub mod indeed;

use crate::config::Config;
use crate::model::Job;
use anyhow::Result;

/// Toda fonte de vagas implementa este trait.
/// Para adicionar uma nova (Gupy, LinkedIn...), crie um módulo e registre em `all()`.
pub trait Source: Send + Sync {
    fn name(&self) -> &'static str;
    async fn fetch(&self, cfg: &Config) -> Result<Vec<Job>>;
}

/// Enum concreto das fontes (evita o problema de `async fn` não ser `dyn` compatível).
pub enum SourceEnum {
    Remotive(remotive::Remotive),
    Wellfound(wellfound::Wellfound),
    Rss(rss::Rss),
    Indeed(indeed::Indeed),
}

impl SourceEnum {
    pub fn name(&self) -> &'static str {
        match self {
            SourceEnum::Remotive(s) => s.name(),
            SourceEnum::Wellfound(s) => s.name(),
            SourceEnum::Rss(s) => s.name(),
            SourceEnum::Indeed(s) => s.name(),
        }
    }

    pub async fn fetch(&self, cfg: &Config) -> Result<Vec<Job>> {
        match self {
            SourceEnum::Remotive(s) => s.fetch(cfg).await,
            SourceEnum::Wellfound(s) => s.fetch(cfg).await,
            SourceEnum::Rss(s) => s.fetch(cfg).await,
            SourceEnum::Indeed(s) => s.fetch(cfg).await,
        }
    }
}

pub fn all() -> Vec<SourceEnum> {
    vec![
        SourceEnum::Remotive(remotive::Remotive),
        SourceEnum::Wellfound(wellfound::Wellfound),
        SourceEnum::Rss(rss::Rss),
        SourceEnum::Indeed(indeed::Indeed),
    ]
}
