use crate::model::Job;
use anyhow::Result;
use rusqlite::{params, Connection};

pub struct Row {
    pub id: i64,
    pub title: String,
    pub company: String,
    pub url: String,
    pub score: i32,
}

pub fn open(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    init(&conn)?;
    Ok(conn)
}

pub fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS jobs (
            id         INTEGER PRIMARY KEY,
            source     TEXT NOT NULL,
            ext_id     TEXT NOT NULL,
            title      TEXT NOT NULL,
            company    TEXT NOT NULL,
            url        TEXT NOT NULL,
            location   TEXT NOT NULL,
            score      INTEGER NOT NULL,
            status     TEXT NOT NULL DEFAULT 'new',
            notified   INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
            UNIQUE(source, ext_id)
        );",
    )?;
    Ok(())
}

/// Devolve true se a vaga era nova (false se já existia).
pub fn insert(conn: &Connection, j: &Job) -> Result<bool> {
    let n = conn.execute(
        "INSERT OR IGNORE INTO jobs (source, ext_id, title, company, url, location, score)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![j.source, j.ext_id, j.title, j.company, j.url, j.location, j.score],
    )?;
    Ok(n > 0)
}

fn map_row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        title: r.get(1)?,
        company: r.get(2)?,
        url: r.get(3)?,
        score: r.get(4)?,
    })
}

pub fn list(conn: &Connection, min_score: i32, status: &str) -> Result<Vec<Row>> {
    let mut st = conn.prepare(
        "SELECT id, title, company, url, score FROM jobs
         WHERE score >= ?1 AND status = ?2
         ORDER BY score DESC, created_at DESC",
    )?;
    let rows = st.query_map(params![min_score, status], map_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn pending_notify(conn: &Connection, min_score: i32) -> Result<Vec<Row>> {
    let mut st = conn.prepare(
        "SELECT id, title, company, url, score FROM jobs
         WHERE notified = 0 AND status = 'new' AND score >= ?1
         ORDER BY score DESC",
    )?;
    let rows = st.query_map(params![min_score], map_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn mark_notified(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("UPDATE jobs SET notified = 1 WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn set_status(conn: &Connection, id: i64, status: &str) -> Result<usize> {
    Ok(conn.execute("UPDATE jobs SET status = ?1 WHERE id = ?2", params![status, id])?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(ext: &str, score: i32) -> Job {
        Job {
            source: "t".into(),
            ext_id: ext.into(),
            title: "Dev".into(),
            company: "Acme".into(),
            url: "http://x".into(),
            location: "Brazil".into(),
            text: String::new(),
            score,
        }
    }

    #[test]
    fn nao_duplica_e_marca_status() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        assert!(insert(&conn, &job("1", 8)).unwrap());
        assert!(!insert(&conn, &job("1", 8)).unwrap());
        assert_eq!(list(&conn, 5, "new").unwrap().len(), 1);
        let id = list(&conn, 5, "new").unwrap()[0].id;
        assert_eq!(set_status(&conn, id, "applied").unwrap(), 1);
        assert!(list(&conn, 5, "new").unwrap().is_empty());
        assert_eq!(list(&conn, 5, "applied").unwrap().len(), 1);
    }

    #[test]
    fn notifica_so_uma_vez() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        insert(&conn, &job("1", 9)).unwrap();
        insert(&conn, &job("2", 2)).unwrap();
        let p = pending_notify(&conn, 6).unwrap();
        assert_eq!(p.len(), 1);
        mark_notified(&conn, p[0].id).unwrap();
        assert!(pending_notify(&conn, 6).unwrap().is_empty());
    }
}
