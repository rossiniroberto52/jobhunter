/// Uma vaga normalizada, independente da fonte de onde veio.
#[derive(Debug, Clone)]
pub struct Job {
    pub source: String,
    pub ext_id: String,
    pub title: String,
    pub company: String,
    pub url: String,
    pub location: String,
    /// Texto usado só para pontuar (título, tags e começo da descrição)
    pub text: String,
    pub score: i32,
}
