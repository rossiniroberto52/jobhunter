mod config;
mod db;
mod model;
mod notify;
mod profile;
mod score;
mod sources;
mod tui;
mod gui;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use futures::future::join_all;

#[derive(Parser)]
#[command(name = "jobhunter", about = "Busca vagas, pontua pelo seu perfil e avisa no Telegram")]
struct Cli {
    #[arg(long, default_value = "config.toml")]
    config: String,
    #[arg(long, default_value = "jobs.db")]
    db: String,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Busca vagas nas fontes, pontua e salva as novas
    Fetch,
    /// Lista vagas salvas (da maior nota para a menor)
    List {
        #[arg(long, default_value_t = 5)]
        min_score: i32,
        #[arg(long, default_value = "new")]
        status: String,
    },
    /// Manda no Telegram as vagas boas que ainda não foram avisadas
    Notify {
        #[arg(long, default_value_t = 6)]
        min_score: i32,
    },
    /// Atualiza o status de uma vaga: applied, interview, rejected, ignored
    Mark { id: i64, status: String },
    /// Abre uma interface de terminal para navegar e marcar vagas
    Tui {
        #[arg(long, default_value_t = 5)]
        min_score: i32,
        #[arg(long, default_value = "new")]
        status: String,
    },
    /// Lê um currículo (txt/md) e sugere pesos para [weights] no config.toml
    Profile {
        /// Caminho do arquivo de currículo
        cv: String,
        /// Arquivo de saída com pesos sugeridos (padrão: weights-from-cv.toml)
        #[arg(long, default_value = "weights-from-cv.toml")]
        out: String,
    },
    /// Abre a interface gráfica (GUI) para navegar e marcar vagas
    Gui,
}

const STATUSES: [&str; 5] = ["new", "applied", "interview", "rejected", "ignored"];

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("erro: {e:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Fetch => {
            let conn = db::open(&cli.db)?;
            let cfg = config::load(&cli.config)?;
            let sources = sources::all();

            // Busca todas as fontes em paralelo
            let futures: Vec<_> = sources
                .into_iter()
                .map(|src| {
                    let cfg = cfg.clone();
                    async move {
                        let name = src.name().to_string();
                        let result = src.fetch(&cfg).await;
                        (name, result)
                    }
                })
                .collect();

            let results = join_all(futures).await;

            let mut jobs = Vec::new();
            for (name, result) in results {
                match result {
                    Ok(mut j) => {
                        println!("[{}] {} vagas recebidas", name, j.len());
                        jobs.append(&mut j);
                    }
                    Err(e) => eprintln!("[{}] erro: {e:#}", name),
                }
            }

            let mut novas = 0;
            for mut job in jobs {
                if !score::location_ok(&job, &cfg) {
                    continue;
                }
                job.score = score::score(&job, &cfg);
                if db::insert(&conn, &job)? {
                    novas += 1;
                }
            }
            println!("{novas} vagas novas salvas");
        }
        Cmd::List { min_score, status } => {
            let conn = db::open(&cli.db)?;
            for r in db::list(&conn, min_score, &status)? {
                println!("#{:<4} [{}/10] {} @ {}\n       {}", r.id, r.score, r.title, r.company, r.url);
            }
        }
        Cmd::Notify { min_score } => {
            let conn = db::open(&cli.db)?;
            let pend = db::pending_notify(&conn, min_score)?;
            if pend.is_empty() {
                println!("nada novo para avisar");
            }
            for r in pend {
                let msg = format!("#{} [{}/10] {} @ {}\n{}", r.id, r.score, r.title, r.company, r.url);
                notify::send(&msg).await?;
                db::mark_notified(&conn, r.id)?;
                println!("avisado: #{}", r.id);
            }
        }
        Cmd::Mark { id, status } => {
            let conn = db::open(&cli.db)?;
            if !STATUSES.contains(&status.as_str()) {
                bail!("status inválido; use um destes: {}", STATUSES.join(", "));
            }
            if db::set_status(&conn, id, &status)? == 0 {
                bail!("vaga #{id} não encontrada");
            }
            println!("vaga #{id} marcada como {status}");
        }
        Cmd::Tui { min_score, status } => {
            let conn = db::open(&cli.db)?;
            tui::run_tui(conn, min_score, status.clone()).map_err(|e| anyhow::anyhow!(e))?;
        }
        Cmd::Profile { cv, out } => {
            let profile = profile::Profile::from_file(&cv)?;
            profile::save_weights_toml(&profile, &out)?;
            println!("currículo lido: {cv}");
            if let Some(level) = &profile.level {
                println!("nível detectado: {level}");
            } else {
                println!("nível detectado: (não identificado)");
            }
            println!("\nTop 15 palavras-chave:");
            for (word, count) in profile.top_keywords(15) {
                println!("  {count}x  {word}");
            }
            println!("\nPesos sugeridos gravados em: {out}");
            println!("Copie as entradas que quiser para [weights] no config.toml.");
        }
        Cmd::Gui => {
            gui::run_gui(cli.db).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
    }
    Ok(())
}
