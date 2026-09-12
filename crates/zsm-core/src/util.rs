use rusqlite::Connection;
use std::collections::HashSet;
use std::path::Path;

pub fn open_ro(path: &Path) -> rusqlite::Result<Connection> {
    Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
}

pub fn open_rw(path: &Path) -> rusqlite::Result<Connection> {
    let con = Connection::open(path)?;
    // wait instead of failing instantly when ZCode holds the write lock briefly
    let _ = con.busy_timeout(std::time::Duration::from_secs(5));
    Ok(con)
}

pub fn existing_tables(con: &Connection) -> HashSet<String> {
    let mut stmt = match con.prepare("SELECT name FROM sqlite_master WHERE type='table'") {
        Ok(s) => s,
        Err(_) => return HashSet::new(),
    };
    let rows = match stmt.query_map([], |r| r.get::<_, String>(0)) {
        Ok(rows) => rows,
        Err(_) => return HashSet::new(),
    };
    rows.filter_map(|r| r.ok()).collect()
}

pub fn has_table(con: &Connection, table: &str) -> bool {
    existing_tables(con).contains(table)
}

pub fn columns(con: &Connection, table: &str) -> HashSet<String> {
    let mut stmt = match con.prepare(&format!("PRAGMA table_info({table})")) {
        Ok(s) => s,
        Err(_) => return HashSet::new(),
    };
    let rows = match stmt.query_map([], |r| r.get::<_, String>(1)) {
        Ok(rows) => rows,
        Err(_) => return HashSet::new(),
    };
    rows.filter_map(|r| r.ok()).collect()
}

pub fn qm(n: usize) -> String {
    vec!["?"; n].join(",")
}
