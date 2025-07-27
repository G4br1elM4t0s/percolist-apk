use rusqlite::{Connection, Result as SqliteResult};
use std::sync::{Arc, Mutex};

pub struct DatabaseState {
    pub connection: Arc<Mutex<Connection>>,
}

pub fn init_database() -> SqliteResult<Connection> {
    let conn = Connection::open("tasks.db")?;

    // Criar tabela original primeiro
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            user TEXT NOT NULL,
            estimated_hours REAL NOT NULL,
            scheduled_date TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL,
            started_at TEXT,
            completed_at TEXT
        )",
        [],
    )?;

    // Migrar banco para adicionar novas colunas se elas não existirem
    migrate_database(&conn)?;

    // Criar outras tabelas
    conn.execute(
        "CREATE TABLE IF NOT EXISTS task_time_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS pomodoro_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            session_number INTEGER NOT NULL,
            session_type TEXT NOT NULL CHECK (session_type IN ('work', 'break')),
            duration_seconds INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS active_sessions (
            task_id INTEGER PRIMARY KEY,
            pomodoro_id INTEGER NOT NULL,
            started_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE,
            FOREIGN KEY (pomodoro_id) REFERENCES pomodoro_sessions (id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS auth_data (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            token TEXT NOT NULL,
            session_id TEXT NOT NULL,
            user_data TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    Ok(conn)
}

fn migrate_database(conn: &Connection) -> SqliteResult<()> {
    // Verificar se as novas colunas existem
    let mut has_description = false;
    let mut has_end_date = false;
    let mut has_should_count = false;
    let mut has_count_value = false;

    // Verificar estrutura da tabela
    let mut stmt = conn.prepare("PRAGMA table_info(tasks)")?;
    let column_iter = stmt.query_map([], |row| {
        Ok(row.get::<_, String>(1)?) // column name
    })?;

    for column_result in column_iter {
        let column_name = column_result?;
        match column_name.as_str() {
            "description" => has_description = true,
            "end_date" => has_end_date = true,
            "should_count" => has_should_count = true,
            "count_value" => has_count_value = true,
            _ => {}
        }
    }

    // Adicionar colunas que não existem
    if !has_description {
        println!("🔄 Adicionando coluna 'description' à tabela tasks");
        conn.execute("ALTER TABLE tasks ADD COLUMN description TEXT", [])?;
    }

    if !has_end_date {
        println!("🔄 Adicionando coluna 'end_date' à tabela tasks");
        conn.execute("ALTER TABLE tasks ADD COLUMN end_date TEXT", [])?;
    }

    if !has_should_count {
        println!("🔄 Adicionando coluna 'should_count' à tabela tasks");
        conn.execute("ALTER TABLE tasks ADD COLUMN should_count BOOLEAN NOT NULL DEFAULT 1", [])?;
    }

    if !has_count_value {
        println!("🔄 Adicionando coluna 'count_value' à tabela tasks");
        conn.execute("ALTER TABLE tasks ADD COLUMN count_value INTEGER NOT NULL DEFAULT 0", [])?;
    }

    println!("✅ Migração do banco de dados concluída");
    Ok(())
}
