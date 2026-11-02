use crate::db::{list, set_status};
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use rusqlite::Connection;
use std::io::{stdout, Result as IoResult};

pub struct App {
    conn: Connection,
    jobs: Vec<crate::db::Row>,
    state: ListState,
    selected: usize,
    min_score: i32,
    status: String,
    message: Option<String>,
}

impl App {
    pub fn new(conn: Connection, min_score: i32, status: String) -> Result<Self> {
        let jobs = list(&conn, min_score, &status)?;
        let mut state = ListState::default();
        if !jobs.is_empty() {
            state.select(Some(0));
        }
        Ok(Self {
            conn,
            jobs,
            state,
            selected: 0,
            min_score,
            status,
            message: None,
        })
    }

    fn refresh(&mut self) -> Result<()> {
        self.jobs = list(&self.conn, self.min_score, &self.status)?;
        if self.selected >= self.jobs.len() && !self.jobs.is_empty() {
            self.selected = self.jobs.len() - 1;
        }
        self.state.select(if self.jobs.is_empty() { None } else { Some(self.selected) });
        Ok(())
    }

    fn set_message(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
    }

    pub fn run(mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = crossterm::event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !self.jobs.is_empty() {
                            self.selected = (self.selected + 1).min(self.jobs.len() - 1);
                            self.state.select(Some(self.selected));
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if !self.jobs.is_empty() {
                            self.selected = self.selected.saturating_sub(1);
                            self.state.select(Some(self.selected));
                        }
                    }
                    KeyCode::Char('a') => {
                        self.mark_applied()?;
                    }
                    KeyCode::Char('i') => {
                        self.mark_status("interview")?;
                    }
                    KeyCode::Char('r') => {
                        self.mark_status("rejected")?;
                    }
                    KeyCode::Char('o') => {
                        self.mark_status("ignored")?;
                    }
                    KeyCode::Char('n') => {
                        self.mark_status("new")?;
                    }
                    KeyCode::Char('+') => {
                        self.min_score += 1;
                        self.refresh()?;
                    }
                    KeyCode::Char('-') => {
                        self.min_score = self.min_score.saturating_sub(1);
                        self.refresh()?;
                    }
                    KeyCode::Char('R') => {
                        self.refresh()?;
                        self.set_message("lista atualizada");
                    }
                    KeyCode::Enter => {}
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn mark_applied(&mut self) -> Result<()> {
        if let Some(id) = self.selected_id() {
            set_status(&self.conn, id, "applied")?;
            self.set_message(format!("vaga #{id} marcada como applied"));
            self.refresh()?;
        }
        Ok(())
    }

    fn mark_status(&mut self, status: &str) -> Result<()> {
        if let Some(id) = self.selected_id() {
            set_status(&self.conn, id, status)?;
            self.set_message(format!("vaga #{id} marcada como {status}"));
            self.refresh()?;
        }
        Ok(())
    }

    fn selected_id(&self) -> Option<i64> {
        self.jobs.get(self.selected).map(|j| j.id)
    }

    fn ui(&mut self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),  // header
                Constraint::Min(0),     // list
                Constraint::Length(3),  // footer/help
            ])
            .split(f.area());

        let title = Paragraph::new(format!(
            "jobhunter — score >= {} | status: {} | {} vagas",
            self.min_score, self.status, self.jobs.len()
        ))
        .block(Block::default().borders(Borders::ALL).title("JobHunter TUI"))
        .style(Style::default().fg(Color::Cyan));
        f.render_widget(title, chunks[0]);

        let rows: Vec<ListItem> = self
            .jobs
            .iter()
            .enumerate()
            .map(|(i, j)| {
                let style = if Some(i) == self.state.selected() {
                    Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                } else if j.score >= 8 {
                    Style::default().fg(Color::Green)
                } else if j.score >= 6 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::raw(format!("#{}  ", j.id)),
                    Span::styled(format!("[{}/10] ", j.score), style),
                    Span::raw(format!("{} @ {}  ", j.title, j.company)),
                    Span::raw(j.url.clone()),
                ]))
                .style(style)
            })
            .collect();

        let list = List::new(rows).block(Block::default().borders(Borders::ALL).title("Vagas"));
        f.render_stateful_widget(list, chunks[1], &mut self.state);

        let help = Paragraph::new(
            "j/k: navega | a: applied | i: interview | r: rejected | o: ignored | n: new | +/-: score | R: atualizar | q: sair",
        )
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
        f.render_widget(help, chunks[2]);

        if let Some(msg) = &self.message {
            let msg_block = Paragraph::new(msg.clone())
                .block(Block::default().borders(Borders::ALL).title("Mensagem"))
                .style(Style::default().fg(Color::Magenta));
            f.render_widget(msg_block, chunks[2]);
        }
    }
}

pub fn run_tui(conn: Connection, min_score: i32, status: String) -> IoResult<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = match App::new(conn, min_score, status) {
        Ok(app) => app,
        Err(e) => {
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
            terminal.show_cursor()?;
            eprintln!("erro ao abrir o TUI: {e}");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
        }
    };

    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("erro no TUI: {e}");
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
    }
    Ok(())
}
