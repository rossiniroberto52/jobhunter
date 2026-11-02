use crate::db::{list, open};
use chrono::Local;
use eframe::egui;
use rfd;
use std::result::Result;
use std::sync::mpsc;
use std::thread;
use webbrowser;

pub struct JobHunterApp {
    db_path: String,
    jobs: Vec<crate::db::Row>,
    min_score: i32,
    status_filter: String,
    log: Vec<String>,
    receiver: Option<mpsc::Receiver<GuiMsg>>,
    sender: mpsc::Sender<GuiMsg>,
    open_url: Option<String>,
    cv_dialog: Option<std::path::PathBuf>,
    cv_result: Option<String>,
}

enum GuiMsg {
    JobsLoaded(Vec<crate::db::Row>),
    Log(String),
    Error(String),
}

impl JobHunterApp {
    pub fn new(db_path: String) -> Self {
        let (sender, receiver) = mpsc::channel();
        let mut app = Self {
            db_path,
            jobs: Vec::new(),
            min_score: 5,
            status_filter: "new".to_string(),
            log: Vec::new(),
            receiver: Some(receiver),
            sender,
            open_url: None,
            cv_dialog: None,
            cv_result: None,
        };
        app.refresh_jobs();
        app
    }

    fn log(&mut self, msg: impl Into<String>) {
        let m = msg.into();
        let line = format!("{} {}", Local::now().format("%H:%M:%S"), m);
        self.log.push(line);
        if self.log.len() > 200 {
            self.log.remove(0);
        }
    }

    fn refresh_jobs(&mut self) {
        let db = self.db_path.clone();
        let min_score = self.min_score;
        let status = self.status_filter.clone();
        let sender = self.sender.clone();

        thread::spawn(move || {
            match open(&db) {
                Ok(conn) => match list(&conn, min_score, &status) {
                    Ok(jobs) => {
                        let _ = sender.send(GuiMsg::JobsLoaded(jobs));
                    }
                    Err(e) => {
                        let _ = sender.send(GuiMsg::Error(format!("erro ao listar: {e}")));
                    }
                },
                Err(e) => {
                    let _ = sender.send(GuiMsg::Error(format!("erro ao abrir DB: {e}")));
                }
            }
        });
    }

    fn fetch_jobs(&mut self) {
        let config_path = "config.toml";
        let db = self.db_path.clone();
        let sender = self.sender.clone();
        let log_sender = self.sender.clone();

        thread::spawn(move || {
            let _ = log_sender.send(GuiMsg::Log("iniciando busca de vagas...".to_string()));

            let cfg = match crate::config::load(config_path) {
                Ok(c) => c,
                Err(e) => {
                    let _ = log_sender.send(GuiMsg::Error(format!(
                        "erro ao carregar config: {e}"
                    )));
                    return;
                }
            };

            let sources = crate::sources::all();
            let mut futures = Vec::new();
            for src in sources {
                let cfg = cfg.clone();
                futures.push(async move {
                    let name = src.name().to_string();
                    let result = src.fetch(&cfg).await;
                    (name, result)
                });
            }

            let runtime = tokio::runtime::Runtime::new().unwrap();
            let results = runtime.block_on(futures::future::join_all(futures));

            let mut jobs = Vec::new();
            for (name, result) in results {
                match result {
                    Ok(mut j) => {
                        let _ = log_sender.send(GuiMsg::Log(format!(
                            "[{}] {} vagas recebidas",
                            name, j.len()
                        )));
                        jobs.append(&mut j);
                    }
                    Err(e) => {
                        let _ = log_sender.send(GuiMsg::Error(format!(
                            "[{}] erro: {e:#}",
                            name
                        )));
                    }
                }
            }

            let mut novas = 0;
            for mut job in jobs {
                if !crate::score::location_ok(&job, &cfg) {
                    continue;
                }
                job.score = crate::score::score(&job, &cfg);
                match crate::db::insert(&open(&db).unwrap(), &job) {
                    Ok(true) => novas += 1,
                    Ok(false) => {}
                    Err(e) => {
                        let _ = log_sender.send(GuiMsg::Error(format!(
                            "erro ao salvar vaga: {e}"
                        )));
                    }
                }
            }

            let _ = log_sender.send(GuiMsg::Log(format!("{novas} vagas novas salvas")));
            let _ = log_sender.send(GuiMsg::Log("atualizando lista...".to_string()));

            match open(&db) {
                Ok(conn) => match list(&conn, 5, "new") {
                    Ok(jobs) => {
                        let _ = sender.send(GuiMsg::JobsLoaded(jobs));
                    }
                    Err(e) => {
                        let _ = sender.send(GuiMsg::Error(format!("erro ao listar: {e}")));
                    }
                },
                Err(e) => {
                    let _ = sender.send(GuiMsg::Error(format!("erro ao abrir DB: {e}")));
                }
            }
        });
    }

    fn notify_jobs(&mut self) {
        let db = self.db_path.clone();
        let sender = self.sender.clone();

        thread::spawn(move || {
            let _ = sender.send(GuiMsg::Log("enviando notificações...".to_string()));

            match open(&db) {
                Ok(conn) => match crate::db::pending_notify(&conn, 6) {
                    Ok(pend) => {
                        if pend.is_empty() {
                            let _ = sender.send(GuiMsg::Log("nada novo para avisar".to_string()));
                            return;
                        }
                        for r in pend {
                            let msg = format!(
                                "#{} [{}/10] {} @ {}\n{}",
                                r.id, r.score, r.title, r.company, r.url
                            );
                            let runtime = tokio::runtime::Runtime::new().unwrap();
                            if let Err(e) = runtime.block_on(crate::notify::send(&msg)) {
                                let _ = sender.send(GuiMsg::Error(format!(
                                    "erro ao avisar #{}: {e}",
                                    r.id
                                )));
                            } else {
                                let _ = crate::db::mark_notified(&conn, r.id);
                                let _ = sender.send(GuiMsg::Log(format!("avisado: #{}", r.id)));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(GuiMsg::Error(format!(
                            "erro ao buscar pendentes: {e}"
                        )));
                    }
                },
                Err(e) => {
                    let _ = sender.send(GuiMsg::Error(format!("erro ao abrir DB: {e}")));
                }
            }
        });
    }

    fn set_job_status(&mut self, id: i64, status: &str) {
        let db = self.db_path.clone();
        let sender = self.sender.clone();
        let status = status.to_string();

        thread::spawn(move || {
            match open(&db) {
                Ok(conn) => {
                    if crate::db::set_status(&conn, id, &status).is_ok() {
                        let _ = sender.send(GuiMsg::Log(format!("vaga #{id} → {status}")));
                    } else {
                        let _ = sender.send(GuiMsg::Error(format!("vaga #{id} não encontrada")));
                    }
                }
                Err(e) => {
                    let _ = sender.send(GuiMsg::Error(format!("erro ao abrir DB: {e}")));
                }
            }
        });
    }

    fn open_url(&mut self, url: &str) {
        self.open_url = Some(url.to_string());
    }

    fn import_cv(&mut self, path: &std::path::Path) {
        // Se for PDF, tenta converter com pdftotext (utilitário do poppler)
        let text = if path.extension().and_then(|e| e.to_str()) == Some("pdf") {
            match converter_pdf_para_texto(path) {
                Ok(t) => t,
                Err(e) => {
                    self.cv_result = Some(format!("{e}"));
                    return;
                }
            }
        } else {
            match std::fs::read_to_string(path) {
                Ok(t) => t,
                Err(e) => {
                    self.cv_result = Some(format!("erro ao ler arquivo: {e}"));
                    return;
                }
            }
        };

        let profile = crate::profile::Profile::from_text(&text);
        let out_path = "pesos-sugeridos.toml";

        match crate::profile::save_weights_toml(&profile, out_path) {
            Ok(()) => {
                let top = profile.top_keywords(10);
                let linhas: Vec<String> = top.iter().map(|(w, c)| format!("  {c}x  {w}")).collect();
                self.cv_result = Some(format!(
                    "currículo importado: {}\nnível: {}\n\nTop keywords:\n{}\n\nPesos salvos em: {}",
                    path.display(),
                    profile.level.as_deref().unwrap_or("(não identificado)"),
                    linhas.join("\n"),
                    out_path
                ));
                self.log(format!("currículo importado: {}", path.display()));
            }
            Err(e) => {
                self.cv_result = Some(format!("erro ao salvar pesos: {e}"));
            }
        }
    }
}

/// Tenta extrair texto de um PDF usando o utilitário `pdftotext` (pacote poppler-utils).
/// Se não estiver instalado, devolve erro com instruções.
fn converter_pdf_para_texto(path: &std::path::Path) -> Result<String, String> {
    let output = std::process::Command::new("pdftotext")
        .arg(path)
        .arg("-")
        .output()
        .map_err(|_| {
            "pdftotext não encontrado. Instale o pacote poppler-utils (Linux) ou poppler (Windows) e tente novamente."
                .to_string()
        })?;

    if !output.status.success() {
        return Err(format!(
            "pdftotext falhou (código {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

impl eframe::App for JobHunterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Processa mensagens assíncronas (escopo separado para evitar borrow)
        if let Some(rx) = self.receiver.take() {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    GuiMsg::JobsLoaded(jobs) => {
                        self.jobs = jobs;
                    }
                    GuiMsg::Log(m) => self.log(m),
                    GuiMsg::Error(m) => self.log(format!("ERRO: {m}")),
                }
            }
            self.receiver = Some(rx);
        }

        // Abre URL no navegador
        if let Some(url) = self.open_url.take() {
            if webbrowser::open(&url).is_err() {
                self.log(format!("não consegui abrir: {url}"));
            }
        }

        // Coleta ações dos botões antes de desenhar (evita borrow conflict)
        let mut open_url: Option<String> = None;
        let mut set_status_actions: Vec<(i64, String)> = Vec::new();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🎯 JobHunter");
            ui.separator();

            // Ações rápidas
            ui.horizontal(|ui| {
                if ui.button("🔍 Buscar vagas").clicked() {
                    self.fetch_jobs();
                }
                if ui.button("📢 Avisar Telegram").clicked() {
                    self.notify_jobs();
                }
                if ui.button("🔄 Atualizar lista").clicked() {
                    self.refresh_jobs();
                }
                if ui.button("📄 Importar currículo").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.cv_dialog = Some(path);
                    }
                }
            });

            ui.separator();

            // Filtros
            ui.horizontal(|ui| {
                ui.label("Score mínimo:");
                ui.add(egui::DragValue::new(&mut self.min_score).speed(1).range(0..=10));
                ui.label("Status:");
                egui::ComboBox::from_label("")
                    .selected_text(&self.status_filter)
                    .show_ui(ui, |ui| {
                        for s in ["new", "applied", "interview", "rejected", "ignored"] {
                            ui.selectable_value(&mut self.status_filter, s.to_string(), s);
                        }
                    });
                if ui.button("Aplicar filtro").clicked() {
                    self.refresh_jobs();
                }
            });

            ui.separator();

            // Lista de vagas
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.jobs.is_empty() {
                    ui.label("Nenhuma vaga encontrada.");
                } else {
                    egui::Grid::new("jobs_grid")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("ID");
                            ui.label("Score");
                            ui.label("Título");
                            ui.label("Empresa");
                            ui.label("Ações");
                            ui.end_row();

                            for job in &self.jobs {
                                let id = job.id;
                                let url = job.url.clone();

                                ui.label(job.id.to_string());
                                ui.label(job.score.to_string());
                                ui.label(&job.title);
                                ui.label(&job.company);

                                ui.horizontal(|ui| {
                                    if ui.button("Abrir").clicked() {
                                        open_url = Some(url.clone());
                                    }
                                    if ui.button("Applied").clicked() {
                                        set_status_actions.push((id, "applied".to_string()));
                                    }
                                    if ui.button("Interview").clicked() {
                                        set_status_actions.push((id, "interview".to_string()));
                                    }
                                    if ui.button("Rejected").clicked() {
                                        set_status_actions.push((id, "rejected".to_string()));
                                    }
                                    if ui.button("Ignored").clicked() {
                                        set_status_actions.push((id, "ignored".to_string()));
                                    }
                                    if ui.button("New").clicked() {
                                        set_status_actions.push((id, "new".to_string()));
                                    }
                                });
                                ui.end_row();
                            }
                        });
                }
            });

            ui.separator();

            // Log
            ui.collapsing("Logs", |ui| {
                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        for line in self.log.iter().rev() {
                            ui.label(line);
                        }
                    });
            });

            ui.separator();

            // Processa currículo selecionado via file dialog
            if let Some(path) = self.cv_dialog.take() {
                self.import_cv(&path);
            }

            // Mostra resultado da importação (colapsável na tela principal)
            if self.cv_result.is_some() {
                ui.separator();
                ui.collapsing("✅ Resultado da importação de currículo", |ui| {
                    if let Some(ref resultado) = self.cv_result {
                        ui.label(resultado);
                    }
                });
            }
            });
        if let Some(url) = open_url {
            self.open_url(&url);
        }
        for (id, status) in set_status_actions {
            self.set_job_status(id, &status);
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(2000));
    }
}

pub fn run_gui(db_path: String) -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_title("JobHunter"),
        ..Default::default()
    };
    eframe::run_native(
        "JobHunter",
        native_options,
        Box::new(|_cc| Ok(Box::new(JobHunterApp::new(db_path)))),
    )
}
