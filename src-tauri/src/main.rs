use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

use tauri::{Manager, PhysicalSize, LogicalPosition, PhysicalPosition, Emitter};
use tauri::State;
use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}, GlobalHotKeyEvent};
use rusqlite::{Connection, Result as SqliteResult, OptionalExtension};
use chrono::{Utc, Datelike};
use serde::{Deserialize, Serialize};

#[cfg(windows)]
use windows::{
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
};

static COLLAPSED_STATE: Mutex<bool> = Mutex::new(false);

static LAST_HOTKEY_TIME: Mutex<Option<Instant>> = Mutex::new(None);

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Task {
    id: Option<i64>,
    name: String,
    description: Option<String>,
    user: String,
    estimated_hours: f64,
    scheduled_date: String,
    end_date: Option<String>,
    status: String,
    created_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    should_count: bool,
    count_value: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TaskTimeLog {
    id: Option<i64>,
    task_id: i64,
    started_at: String,
    ended_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PomodoroSession {
    id: Option<i64>,
    task_id: i64,
    session_number: i32,
    session_type: String, // "work" or "break"
    duration_seconds: i32,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ActiveSession {
    task_id: i64,
    pomodoro_id: i64,
    started_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TaskWithActiveSession {
    id: Option<i64>,
    name: String,
    description: Option<String>,
    user: String,
    estimated_hours: f64,
    scheduled_date: String,
    end_date: Option<String>,
    status: String,
    created_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    should_count: bool,
    count_value: u32,
    active_session: Option<ActiveSessionInfo>,
    pomodoro_sessions: Vec<PomodoroSessionInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PomodoroSessionInfo {
    id: Option<i64>,
    session_number: i32,
    session_type: String,
    duration_seconds: i32,
    created_at: String,
    is_active: bool,
    started_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ActiveSessionInfo {
    session_type: String,
    started_at: String,
    ends_at: String,
    duration_seconds: i32,
}

struct DatabaseState {
    connection: Arc<Mutex<Connection>>,
}

fn init_database() -> SqliteResult<Connection> {
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

fn get_next_business_day(date: chrono::NaiveDate) -> chrono::NaiveDate {
    let mut next_day = date + chrono::Duration::days(1);

    // Pular fins de semana (sábado = 6, domingo = 0)
    while next_day.weekday().num_days_from_monday() >= 5 {
        next_day = next_day + chrono::Duration::days(1);
    }

    next_day
}

fn calculate_total_worked_seconds(conn: &Connection, task_id: i64) -> Result<i64, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT started_at, ended_at FROM task_time_logs WHERE task_id = ?1"
    )?;

    let log_iter = stmt.query_map([task_id], |row| {
        Ok((
            row.get::<_, String>(0)?, // started_at
            row.get::<_, Option<String>>(1)?, // ended_at
        ))
    })?;

    let mut total_seconds = 0i64;
    let now = chrono::Utc::now();

    for log_result in log_iter {
        let (started_at_str, ended_at_opt) = log_result?;

        let started_at = chrono::DateTime::parse_from_rfc3339(&started_at_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "started_at".to_string(), rusqlite::types::Type::Text))?;

        let ended_at = match ended_at_opt {
            Some(ended_at_str) => {
                chrono::DateTime::parse_from_rfc3339(&ended_at_str)
                    .map_err(|_| rusqlite::Error::InvalidColumnType(1, "ended_at".to_string(), rusqlite::types::Type::Text))?
            }
            None => now.into() // Se ainda está ativo, usar tempo atual
        };

        let duration = ended_at.signed_duration_since(started_at);
        total_seconds += duration.num_seconds();
    }

    Ok(total_seconds)
}

fn reschedule_incomplete_tasks(conn: &Connection) -> Result<Vec<i64>, rusqlite::Error> {
    let today = chrono::Local::now().date_naive();
    let yesterday = today - chrono::Duration::days(1);
    let next_business_day = get_next_business_day(today);

    println!("🔄 Verificando tarefas para remanejar de {} para {}", yesterday, next_business_day);

    // Buscar tarefas elegíveis para remanejamento:
    // 1. scheduled_date <= ontem
    // 2. end_date IS NULL (sem data final definida)
    // 3. status != 'completed'
    // 4. estimated_hours > 0
    let mut stmt = conn.prepare(
        "SELECT id, name, estimated_hours, scheduled_date
         FROM tasks
         WHERE scheduled_date <= ?1
           AND end_date IS NULL
           AND status != 'completed'
           AND estimated_hours > 0
         ORDER BY scheduled_date ASC"
    )?;

    let yesterday_str = yesterday.format("%Y-%m-%d").to_string();
    let task_candidates: Vec<(i64, String, f64, String)> = stmt.query_map([&yesterday_str], |row| {
        Ok((
            row.get(0)?, // id
            row.get(1)?, // name
            row.get(2)?, // estimated_hours
            row.get(3)?, // scheduled_date
        ))
    })?.collect::<Result<Vec<_>, _>>()?;

    let mut rescheduled_tasks = Vec::new();

    for (task_id, task_name, estimated_hours, scheduled_date) in task_candidates {
        // Calcular tempo total trabalhado
        let total_worked_seconds = calculate_total_worked_seconds(conn, task_id)?;
        let estimated_seconds = (estimated_hours * 3600.0) as i64;
        let remaining_seconds = estimated_seconds - total_worked_seconds;

        println!("📊 Tarefa {}: {} - Estimado: {}h, Trabalhado: {}s, Restante: {}s",
            task_id, task_name, estimated_hours, total_worked_seconds, remaining_seconds);

        // Se ainda há tempo restante significativo (mais de 1 minuto), remanejar
        if remaining_seconds > 60 {
            let next_day_str = next_business_day.format("%Y-%m-%d").to_string();

            // Atualizar scheduled_date para próximo dia útil
            conn.execute(
                "UPDATE tasks SET scheduled_date = ?1 WHERE id = ?2",
                rusqlite::params![&next_day_str, task_id],
            )?;

            println!("📅 Tarefa {} remanejada de {} para {}", task_id, scheduled_date, next_day_str);
            rescheduled_tasks.push(task_id);
        } else {
            println!("✅ Tarefa {} quase concluída, não remanejando", task_id);
        }
    }

    if !rescheduled_tasks.is_empty() {
        println!("🔄 {} tarefas remanejadas para {}", rescheduled_tasks.len(), next_business_day);
    } else {
        println!("✅ Nenhuma tarefa precisou ser remanejada");
    }

    Ok(rescheduled_tasks)
}

fn debug_task_time_logs(conn: &Connection, task_id: i64) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, started_at, ended_at FROM task_time_logs WHERE task_id = ?1 ORDER BY started_at"
    )?;

    let log_iter = stmt.query_map([task_id], |row| {
        Ok((
            row.get::<_, i64>(0)?, // id
            row.get::<_, String>(1)?, // started_at
            row.get::<_, Option<String>>(2)?, // ended_at
        ))
    })?;

    println!("🔍 Logs de tempo para tarefa {}:", task_id);
    for log_result in log_iter {
        let (log_id, started_at, ended_at) = log_result?;
        match ended_at {
            Some(ended) => println!("  📝 Log {}: {} → {} (finalizado)", log_id, started_at, ended),
            None => println!("  ⏳ Log {}: {} → (ativo)", log_id, started_at),
        }
    }

    Ok(())
}

fn calculate_task_remaining_time(conn: &Connection, task_id: i64, estimated_hours: f64) -> Result<i64, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT started_at, ended_at FROM task_time_logs WHERE task_id = ?1 ORDER BY started_at"
    )?;

    let log_iter = stmt.query_map([task_id], |row| {
        Ok((
            row.get::<_, String>(0)?, // started_at
            row.get::<_, Option<String>>(1)?, // ended_at
        ))
    })?;

    let mut total_seconds_worked = 0i64;
    let now = Utc::now();

    println!("🔍 Calculando tempo para tarefa {}: estimated_hours = {}", task_id, estimated_hours);

    for log_result in log_iter {
        let (started_at_str, ended_at_opt) = log_result?;

        let started_at = chrono::DateTime::parse_from_rfc3339(&started_at_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "started_at".to_string(), rusqlite::types::Type::Text))?;

        let ended_at = match ended_at_opt {
            Some(ended_at_str) => {
                let ended_time = chrono::DateTime::parse_from_rfc3339(&ended_at_str)
                    .map_err(|_| rusqlite::Error::InvalidColumnType(1, "ended_at".to_string(), rusqlite::types::Type::Text))?;
                println!("📝 Log completo: {} → {} (finalizado)", started_at_str, ended_at_str);
                ended_time
            }
            None => {
                println!("⏳ Log ativo: {} → agora (em andamento)", started_at_str);
                now.into() // Se não tem ended_at, significa que está ativo, usa tempo atual
            }
        };

        let duration = ended_at.signed_duration_since(started_at);
        let duration_seconds = duration.num_seconds();
        total_seconds_worked += duration_seconds;

        println!("⏱️ Duração deste período: {}s", duration_seconds);
    }

    let estimated_seconds = (estimated_hours * 3600.0) as i64;
    let remaining_seconds = estimated_seconds - total_seconds_worked;

    println!("📊 Total trabalhado: {}s, Estimado: {}s, Restante: {}s",
             total_seconds_worked, estimated_seconds, remaining_seconds);

    Ok(remaining_seconds)
}

fn create_pomodoro_cycles(conn: &Connection, task_id: i64) -> Result<(), rusqlite::Error> {
    // Criar ciclo padrão Pomodoro: 25min trabalho, 5min pausa, repetir 4x, depois 15min pausa longa
    let cycles = [
        ("work", 25 * 60),    // 25 min trabalho
        ("break", 5 * 60),    // 5 min pausa
        ("work", 25 * 60),    // 25 min trabalho
        ("break", 5 * 60),    // 5 min pausa
        ("work", 25 * 60),    // 25 min trabalho
        ("break", 5 * 60),    // 5 min pausa
        ("work", 25 * 60),    // 25 min trabalho
        ("break", 15 * 60),   // 15 min pausa longa
    ];

    let now = Utc::now().to_rfc3339();

    for (i, (session_type, duration)) in cycles.iter().enumerate() {
        conn.execute(
            "INSERT INTO pomodoro_sessions (task_id, session_number, session_type, duration_seconds, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [
                &task_id.to_string(),
                &(i + 1).to_string(),
                &session_type.to_string(),
                &duration.to_string(),
                &now
            ],
        )?;
    }

    Ok(())
}

fn get_next_pomodoro_session(conn: &Connection, task_id: i64) -> Result<Option<PomodoroSession>, rusqlite::Error> {
    // Verificar se já existem sessões para esta tarefa
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pomodoro_sessions WHERE task_id = ?1",
        [task_id],
        |row| row.get(0),
    )?;

    // Se não existem sessões, criar os ciclos
    if count == 0 {
        create_pomodoro_cycles(conn, task_id)?;
    }

        // Buscar a próxima sessão disponível (menor session_number que não está em uso)
    let mut stmt = conn.prepare(
        "SELECT ps.id, ps.task_id, ps.session_number, ps.session_type, ps.duration_seconds, ps.created_at
         FROM pomodoro_sessions ps
         WHERE ps.task_id = ?1 AND ps.id NOT IN (
             SELECT DISTINCT pomodoro_id FROM active_sessions WHERE task_id = ?1
         )
         ORDER BY ps.session_number ASC
         LIMIT 1"
    )?;

    let session_opt = stmt.query_row([task_id], |row| {
        Ok(PomodoroSession {
            id: Some(row.get(0)?),
            task_id: row.get(1)?,
            session_number: row.get(2)?,
            session_type: row.get(3)?,
            duration_seconds: row.get(4)?,
            created_at: row.get(5)?,
        })
    }).optional()?;

    Ok(session_opt)
}

fn start_pomodoro_session(conn: &Connection, task_id: i64, pomodoro_session: &PomodoroSession) -> Result<String, rusqlite::Error> {
    let now = Utc::now().to_rfc3339();

    // Inserir sessão ativa
    conn.execute(
        "INSERT OR REPLACE INTO active_sessions (task_id, pomodoro_id, started_at) VALUES (?1, ?2, ?3)",
        [&task_id.to_string(), &pomodoro_session.id.unwrap().to_string(), &now],
    )?;

    // Determinar status com base no tipo de sessão
    let status = match pomodoro_session.session_type.as_str() {
        "work" => "in_progress",
        "break" => "waiting",
        _ => "in_progress",
    };

    // Atualizar status da tarefa
    conn.execute(
        "UPDATE tasks SET status = ?1 WHERE id = ?2",
        [status, &task_id.to_string()],
    )?;

    Ok(status.to_string())
}

fn check_and_advance_pomodoro_sessions(conn: &Connection) -> Result<Vec<i64>, rusqlite::Error> {
    let now = Utc::now();
    let mut advanced_tasks = Vec::new();

    // Buscar sessões ativas que ultrapassaram o tempo
    let mut stmt = conn.prepare(
        "SELECT a.task_id, a.pomodoro_id, a.started_at, p.duration_seconds, p.session_type
         FROM active_sessions a
         JOIN pomodoro_sessions p ON a.pomodoro_id = p.id"
    )?;

    let rows: Vec<(i64, i64, String, i32, String)> = stmt.query_map([], |row| {
        Ok((
            row.get(0)?, // task_id
            row.get(1)?, // pomodoro_id
            row.get(2)?, // started_at
            row.get(3)?, // duration_seconds
            row.get(4)?, // session_type
        ))
    })?.collect::<Result<Vec<_>, _>>()?;

    for (task_id, pomodoro_id, started_at_str, duration_seconds, session_type) in rows {
        let started_at = chrono::DateTime::parse_from_rfc3339(&started_at_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "started_at".to_string(), rusqlite::types::Type::Text))?;

        let elapsed = now.signed_duration_since(started_at);
        let elapsed_seconds = elapsed.num_seconds();

        // Se ultrapassou o tempo da sessão
        if elapsed_seconds >= duration_seconds as i64 {
            println!("Sessão {} da tarefa {} ultrapassou tempo: {}s >= {}s",
                pomodoro_id, task_id, elapsed_seconds, duration_seconds);

            // Remover sessão ativa atual
            conn.execute(
                "DELETE FROM active_sessions WHERE task_id = ?1",
                [task_id],
            )?;

            // Finalizar log de tempo se for sessão de trabalho E se ainda não foi finalizado
            if session_type == "work" {
                // Verificar se há log ativo (não finalizado) para esta tarefa
                let active_log_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM task_time_logs WHERE task_id = ?1 AND ended_at IS NULL",
                    [task_id],
                    |row| row.get(0),
                )?;

                // Só finalizar se realmente há um log ativo (não foi pausado manualmente)
                if active_log_count > 0 {
                    let session_end_time = started_at + chrono::Duration::seconds(duration_seconds as i64);
                    conn.execute(
                        "UPDATE task_time_logs SET ended_at = ?1 WHERE task_id = ?2 AND ended_at IS NULL",
                        [&session_end_time.to_rfc3339(), &task_id.to_string()],
                    )?;
                    println!("🕐 Log de tempo finalizado automaticamente para tarefa {} às {}", task_id, session_end_time.to_rfc3339());
                } else {
                    println!("⚠️ Log já foi finalizado manualmente para tarefa {}, não sobrescrever", task_id);
                }
            }

            // Buscar próxima sessão
            let next_session = get_next_pomodoro_session(conn, task_id)?;

            match next_session {
                Some(next_pomodoro) => {
                    // Iniciar próxima sessão automaticamente
                    start_pomodoro_session(conn, task_id, &next_pomodoro)?;
                    advanced_tasks.push(task_id);

                    println!("Tarefa {} avançou para sessão: {} ({})",
                        task_id, next_pomodoro.session_type, next_pomodoro.session_number);
                }
                None => {
                    // Não há mais sessões, completar tarefa
                    let now_str = now.to_rfc3339();
                    conn.execute(
                        "UPDATE tasks SET status = 'completed', completed_at = ?1 WHERE id = ?2",
                        [&now_str, &task_id.to_string()],
                    )?;
                    advanced_tasks.push(task_id);

                    println!("Tarefa {} completada automaticamente - todos os Pomodoros finalizados", task_id);
                }
            }
        }
    }

    Ok(advanced_tasks)
}

#[tauri::command]
async fn toggle_collapse(window: tauri::WebviewWindow, is_collapsed: bool) -> Result<(), String> {
    println!("🔧 toggle_collapse chamado com is_collapsed: {}", is_collapsed);

    // Atualizar o estado global
    if let Ok(mut state) = COLLAPSED_STATE.lock() {
        *state = is_collapsed;
        println!("🔧 Estado global atualizado para: {}", *state);
    }

    let new_height = if is_collapsed { 1 } else { 70 };
    println!("🔧 Tentando redimensionar janela para altura: {}", new_height);

    // Primeiro, tentar redimensionar
    match window.set_size(PhysicalSize::new(1920, new_height)) {
        Ok(_) => println!("✓ Janela redimensionada com sucesso para {}px", new_height),
        Err(e) => {
            println!("✗ Erro ao redimensionar janela: {}", e);
            return Err(e.to_string());
        }
    }

    // Se colapsada, também mover para posição específica
    if is_collapsed {
        println!("🔧 Movendo janela colapsada para posição (-1, -1)");
        match window.set_position(LogicalPosition::new(-06.5, -1.0)) {
            Ok(_) => println!("✓ Janela movida para posição colapsada"),
            Err(e) => println!("✗ Erro ao mover janela: {}", e),
        }
    } else {
        println!("🔧 Restaurando janela para posição (0, 0)");
        match window.set_position(LogicalPosition::new(-06.5, -1.0)) {
            Ok(_) => println!("✓ Janela restaurada para posição normal"),
            Err(e) => println!("✗ Erro ao restaurar janela: {}", e),
        }
    }

    // Aguardar um pouco e verificar o tamanho atual
    thread::sleep(Duration::from_millis(100));

    match window.inner_size() {
        Ok(size) => println!("🔧 Tamanho atual da janela: {}x{}", size.width, size.height),
        Err(e) => println!("✗ Erro ao obter tamanho da janela: {}", e),
    }

    println!("🔧 toggle_collapse finalizado");
    Ok(())
}

#[tauri::command]
async fn get_collapsed_state() -> Result<bool, String> {
    let state = COLLAPSED_STATE.lock()
        .map(|state| *state)
        .map_err(|e| e.to_string())?;
    println!("🔧 get_collapsed_state retornando: {}", state);
    Ok(state)
}

#[tauri::command]
async fn test_hotkey_manually(window: tauri::WebviewWindow) -> Result<(), String> {
    println!("🔧 Teste manual do atalho executado!");

    // Alternar estado manualmente
    let new_state = if let Ok(state) = COLLAPSED_STATE.lock() {
        !*state
    } else {
        false
    };

    println!("🔧 Novo estado será: {}", new_state);

    // Chamar toggle_collapse para aplicar as mudanças
    toggle_collapse(window, new_state).await?;

    Ok(())
}

fn should_process_hotkey() -> bool {
    if let Ok(mut last_time) = LAST_HOTKEY_TIME.lock() {
        let now = Instant::now();

        if let Some(last) = *last_time {
            // Se passou menos de 500ms desde o último atalho, ignorar
            if now.duration_since(last) < Duration::from_millis(500) {
                println!("🔧 Atalho ignorado (debounce)");
                return false;
            }
        }

        *last_time = Some(now);
        true
    } else {
        false
    }
}

#[tauri::command]
async fn get_system_volume() -> Result<i32, String> {
    #[cfg(target_os = "linux")]
    {
        // Usar pactl para obter o volume no Linux
        match Command::new("pactl")
            .args(["get-sink-volume", "@DEFAULT_SINK@"])
            .output()
        {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Procurar por porcentagem no formato "XX%"
                for line in output_str.lines() {
                    if let Some(percent_pos) = line.find('%') {
                        // Procurar o número antes do %
                        let before_percent = &line[..percent_pos];
                        if let Some(space_pos) = before_percent.rfind(' ') {
                            let volume_str = &before_percent[space_pos + 1..];
                            if let Ok(volume) = volume_str.parse::<i32>() {
                                return Ok(volume);
                            }
                        }
                    }
                }
                Ok(50) // Fallback
            }
            Err(_) => Ok(50) // Fallback se pactl não estiver disponível
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Para outros sistemas, retornar um valor padrão por enquanto
        Ok(50)
    }
}

#[tauri::command]
async fn set_system_volume(volume: i32) -> Result<(), String> {
    let clamped_volume = volume.clamp(0, 100);

    #[cfg(target_os = "linux")]
    {
        // Usar pactl para definir o volume no Linux
        match Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", clamped_volume)])
            .output()
        {
            Ok(_) => {
                println!("🔊 Volume definido para {}%", clamped_volume);
                Ok(())
            }
            Err(e) => {
                println!("✗ Erro ao definir volume: {}", e);
                Err(format!("Erro ao definir volume: {}", e))
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("🔊 Volume seria definido para {}% (não implementado para este OS)", clamped_volume);
        Ok(())
    }
}

#[tauri::command]
async fn get_system_mute_status() -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    {
        match Command::new("pactl")
            .args(["get-sink-mute", "@DEFAULT_SINK@"])
            .output()
        {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                Ok(output_str.trim() == "yes")
            }
            Err(_) => Ok(false)
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(false)
    }
}

#[tauri::command]
async fn toggle_system_mute() -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    {
        match Command::new("pactl")
            .args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
            .output()
        {
            Ok(_) => {
                // Obter o novo status após o toggle
                get_system_mute_status().await
            }
            Err(e) => Err(format!("Erro ao alternar mute: {}", e))
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(false)
    }
}

#[tauri::command]
async fn load_tasks(db_state: State<'_, DatabaseState>) -> Result<Vec<Task>, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT id, name, description, user, estimated_hours, scheduled_date, end_date, status, created_at, started_at, completed_at, should_count, count_value
         FROM tasks ORDER BY scheduled_date ASC, created_at ASC"
    ).map_err(|e| e.to_string())?;

    let task_iter = stmt.query_map([], |row| {
        Ok(Task {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            description: row.get(2)?,
            user: row.get(3)?,
            estimated_hours: row.get(4)?,
            scheduled_date: row.get(5)?,
            end_date: row.get(6)?,
            status: row.get(7)?,
            created_at: row.get(8)?,
            started_at: row.get(9)?,
            completed_at: row.get(10)?,
            should_count: row.get(11)?,
            count_value: row.get(12)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut tasks = Vec::new();
    for task in task_iter {
        tasks.push(task.map_err(|e| e.to_string())?);
    }

    Ok(tasks)
}

#[tauri::command]
async fn add_task(
    name: String,
    description: Option<String>,
    user: String,
    estimated_hours: f64,
    scheduled_date: String,
    end_date: Option<String>,
    should_count: bool,
    count_value: u32,
    db_state: State<'_, DatabaseState>
) -> Result<Task, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO tasks (name, description, user, estimated_hours, scheduled_date, end_date, status, created_at, should_count, count_value)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?8, ?9)",
        rusqlite::params![
            &name,
            description.as_deref(),
            &user,
            estimated_hours,
            &scheduled_date,
            end_date.as_deref(),
            &now,
            should_count,
            count_value
        ],
    ).map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    // Criar sessões Pomodoro automaticamente quando a tarefa é criada
    create_pomodoro_cycles(&conn, id).map_err(|e| e.to_string())?;
    println!("🍅 Sessões Pomodoro criadas automaticamente para tarefa {}", id);

    Ok(Task {
        id: Some(id),
        name,
        description,
        user,
        estimated_hours,
        scheduled_date,
        end_date,
        status: "pending".to_string(),
        created_at: now,
        started_at: None,
        completed_at: None,
        should_count,
        count_value,
    })
}

#[tauri::command]
async fn start_task(task_id: i64, stop_and_start: Option<bool>, db_state: State<'_, DatabaseState>) -> Result<(), String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    println!("🔧 Iniciando tarefa: {}", task_id);
    // Verificar se já existe uma tarefa ativa (sem ended_at)
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM task_time_logs WHERE task_id = ?1 AND ended_at IS NULL"
    ).map_err(|e| e.to_string())?;

    let count: i64 = stmt.query_row([task_id], |row| row.get(0)).map_err(|e| e.to_string())?;

    if count > 0 {
        return Err("Tarefa já está ativa".to_string());
    }

    // NOVA REGRA: Verificar se há alguma outra tarefa em andamento
    let active_tasks_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE status IN ('in_progress', 'waiting') AND id != ?1",
        [task_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    // Se stop_and_start for true, pausar automaticamente tarefas ativas
    if active_tasks_count > 0 {
        if stop_and_start.unwrap_or(false) {
            println!("🔄 stop_and_start=true: pausando tarefas ativas automaticamente");

            // Buscar IDs das tarefas ativas
            let mut stmt = conn.prepare(
                "SELECT id FROM tasks WHERE status IN ('in_progress', 'waiting') AND id != ?1"
            ).map_err(|e| e.to_string())?;

            let active_task_ids: Vec<i64> = stmt.query_map([task_id], |row| {
                Ok(row.get::<_, i64>(0)?)
            }).map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

            // Pausar cada tarefa ativa
            for active_task_id in active_task_ids {
                println!("🛑 Pausando tarefa ativa: {}", active_task_id);

                // Remover sessão ativa
                conn.execute(
                    "DELETE FROM active_sessions WHERE task_id = ?1",
                    [active_task_id],
                ).map_err(|e| e.to_string())?;

                // Finalizar log de tempo se existir
                conn.execute(
                    "UPDATE task_time_logs SET ended_at = ?1 WHERE task_id = ?2 AND ended_at IS NULL",
                    [&now, &active_task_id.to_string()],
                ).map_err(|e| e.to_string())?;

                // Atualizar status para 'paused'
                conn.execute(
                    "UPDATE tasks SET status = 'paused' WHERE id = ?1",
                    [&active_task_id.to_string()],
                ).map_err(|e| e.to_string())?;

                println!("✅ Tarefa {} pausada automaticamente", active_task_id);
            }
        } else {
            return Err("Apenas uma tarefa pode estar em andamento por vez. Pause a tarefa atual primeiro.".to_string());
        }
    }

    // Buscar próxima sessão Pomodoro
    let next_session = get_next_pomodoro_session(&conn, task_id)
        .map_err(|e| e.to_string())?;

        match &next_session {
        Some(pomodoro_session) => {
            // Iniciar sessão Pomodoro
            let status = start_pomodoro_session(&conn, task_id, pomodoro_session)
                .map_err(|e| e.to_string())?;

            // Atualizar started_at apenas se for a primeira vez
            let mut stmt = conn.prepare("SELECT started_at FROM tasks WHERE id = ?1")
                .map_err(|e| e.to_string())?;
            let current_started_at: Option<String> = stmt.query_row([task_id], |row| row.get(0))
                .map_err(|e| e.to_string())?;

            if current_started_at.is_none() {
                conn.execute(
                    "UPDATE tasks SET started_at = ?1 WHERE id = ?2",
                    [&now, &task_id.to_string()],
                ).map_err(|e| e.to_string())?;
            }

            println!("Tarefa {} iniciada com sessão Pomodoro: {} ({})",
                task_id, pomodoro_session.session_type, status);
        }
        None => {
            // Não há mais sessões Pomodoro, marcar como completada
            conn.execute(
                "UPDATE tasks SET status = 'completed', completed_at = ?1 WHERE id = ?2",
                [&now, &task_id.to_string()],
            ).map_err(|e| e.to_string())?;

            println!("Tarefa {} completada - todos os ciclos Pomodoro finalizados", task_id);
        }
    }

    // Criar novo log de tempo (apenas para sessões de trabalho)
    if let Some(session) = &next_session {
        if session.session_type == "work" {
            conn.execute(
                "INSERT INTO task_time_logs (task_id, started_at) VALUES (?1, ?2)",
                [&task_id.to_string(), &now],
            ).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
async fn check_pomodoro_sessions(db_state: State<'_, DatabaseState>) -> Result<Vec<i64>, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    let advanced_tasks = check_and_advance_pomodoro_sessions(&conn)
        .map_err(|e| e.to_string())?;

    Ok(advanced_tasks)
}

#[tauri::command]
async fn load_tasks_with_sessions(db_state: State<'_, DatabaseState>) -> Result<Vec<TaskWithActiveSession>, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.description, t.user, t.estimated_hours, t.scheduled_date, t.end_date, t.status,
                t.created_at, t.started_at, t.completed_at, t.should_count, t.count_value,
                a.started_at as session_started_at, p.session_type, p.duration_seconds
         FROM tasks t
         LEFT JOIN active_sessions a ON t.id = a.task_id
         LEFT JOIN pomodoro_sessions p ON a.pomodoro_id = p.id
         ORDER BY
            CASE
                WHEN t.status IN ('in_progress', 'waiting') THEN 0
                ELSE 1
            END ASC,
            t.scheduled_date ASC,
            t.created_at ASC"
    ).map_err(|e| e.to_string())?;

    let task_iter = stmt.query_map([], |row| {
        let task_id: i64 = row.get(0)?;
        let session_started_at: Option<String> = row.get(13)?;
        let session_type: Option<String> = row.get(14)?;
        let duration_seconds: Option<i32> = row.get(15)?;

        let active_session = if let (Some(started_at), Some(s_type), Some(duration)) =
            (session_started_at, session_type, duration_seconds) {

            let started_time = chrono::DateTime::parse_from_rfc3339(&started_at)
                .map_err(|_| rusqlite::Error::InvalidColumnType(13, "session_started_at".to_string(), rusqlite::types::Type::Text))?;
            let ends_at = started_time + chrono::Duration::seconds(duration as i64);

            Some(ActiveSessionInfo {
                session_type: s_type,
                started_at,
                ends_at: ends_at.to_rfc3339(),
                duration_seconds: duration,
            })
        } else {
            None
        };

        Ok(TaskWithActiveSession {
            id: Some(task_id),
            name: row.get(1)?,
            description: row.get(2)?,
            user: row.get(3)?,
            estimated_hours: row.get(4)?,
            scheduled_date: row.get(5)?,
            end_date: row.get(6)?,
            status: row.get(7)?,
            created_at: row.get(8)?,
            started_at: row.get(9)?,
            completed_at: row.get(10)?,
            should_count: row.get(11)?,
            count_value: row.get(12)?,
            active_session,
            pomodoro_sessions: Vec::new(), // Será preenchido depois
        })
    }).map_err(|e| e.to_string())?;

    let mut tasks = Vec::new();
    for task in task_iter {
        tasks.push(task.map_err(|e| e.to_string())?);
    }

    // Agora, carregar todas as sessões Pomodoro para cada task
    for task in &mut tasks {
        if let Some(task_id) = task.id {
            let mut pomodoro_stmt = conn.prepare(
                "SELECT ps.id, ps.session_number, ps.session_type, ps.duration_seconds, ps.created_at,
                        a.started_at as active_started_at
                 FROM pomodoro_sessions ps
                 LEFT JOIN active_sessions a ON ps.id = a.pomodoro_id AND ps.task_id = a.task_id
                 WHERE ps.task_id = ?1
                 ORDER BY ps.session_number ASC"
            ).map_err(|e| e.to_string())?;

            let pomodoro_iter = pomodoro_stmt.query_map([task_id], |row| {
                let active_started_at: Option<String> = row.get(5)?;
                Ok(PomodoroSessionInfo {
                    id: Some(row.get(0)?),
                    session_number: row.get(1)?,
                    session_type: row.get(2)?,
                    duration_seconds: row.get(3)?,
                    created_at: row.get(4)?,
                    is_active: active_started_at.is_some(),
                    started_at: active_started_at,
                })
            }).map_err(|e| e.to_string())?;

                        let mut pomodoro_sessions = Vec::new();
            for session in pomodoro_iter {
                pomodoro_sessions.push(session.map_err(|e| e.to_string())?);
            }

            println!("🍅 Tarefa {} ({}) carregou {} sessões Pomodoro",
                task_id, task.name, pomodoro_sessions.len());

            task.pomodoro_sessions = pomodoro_sessions;
        }
    }

    Ok(tasks)
}

#[tauri::command]
async fn complete_task(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<(), String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();

    // Remover sessão ativa se existir
    conn.execute(
        "DELETE FROM active_sessions WHERE task_id = ?1",
        [task_id],
    ).map_err(|e| e.to_string())?;

    // Finalizar log ativo se existir
    conn.execute(
        "UPDATE task_time_logs SET ended_at = ?1 WHERE task_id = ?2 AND ended_at IS NULL",
        [&now, &task_id.to_string()],
    ).map_err(|e| e.to_string())?;

    // Atualizar status da tarefa
    conn.execute(
        "UPDATE tasks SET status = 'completed', completed_at = ?1 WHERE id = ?2",
        [&now, &task_id.to_string()],
    ).map_err(|e| e.to_string())?;

    println!("Tarefa {} completada manualmente", task_id);
    Ok(())
}

#[tauri::command]
async fn pause_task(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<(), String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();

    // Verificar se há sessão ativa
    let active_session_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM active_sessions WHERE task_id = ?1",
        [task_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    if active_session_exists == 0 {
        return Err("Nenhuma sessão Pomodoro ativa encontrada para pausar".to_string());
    }

    // Finalizar log de tempo ANTES de remover sessão ativa (para evitar conflito com check_and_advance)
    let rows_updated = conn.execute(
        "UPDATE task_time_logs SET ended_at = ?1 WHERE task_id = ?2 AND ended_at IS NULL",
        [&now, &task_id.to_string()],
    ).map_err(|e| e.to_string())?;

    println!("⏸️ Pausando tarefa {} às {} - {} logs finalizados", task_id, now, rows_updated);

    // Debug: mostrar logs após pausar
    let _ = debug_task_time_logs(&conn, task_id);

    // Remover sessão ativa (pausa o Pomodoro) - fazer isso por último
    conn.execute(
        "DELETE FROM active_sessions WHERE task_id = ?1",
        [task_id],
    ).map_err(|e| e.to_string())?;

    // Atualizar status da tarefa para 'paused'
    conn.execute(
        "UPDATE tasks SET status = 'paused' WHERE id = ?1",
        [&task_id.to_string()],
    ).map_err(|e| e.to_string())?;

    println!("Tarefa {} pausada - sessão Pomodoro interrompida", task_id);
    Ok(())
}


#[tauri::command]
async fn resume_task(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<(), String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    // Verificar se a tarefa existe e está pausada
    let mut stmt = conn.prepare(
        "SELECT status FROM tasks WHERE id = ?1"
    ).map_err(|e| e.to_string())?;

    let status: String = stmt.query_row([task_id], |row| row.get(0))
        .map_err(|_| "Tarefa não encontrada".to_string())?;

    if status != "paused" {
        return Err("Tarefa não está pausada".to_string());
    }

    // NOVA REGRA: Verificar se há alguma outra tarefa em andamento
    let active_tasks_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE status IN ('in_progress', 'waiting') AND id != ?1",
        [task_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    if active_tasks_count > 0 {
        return Err("Apenas uma tarefa pode estar em andamento por vez. Pause a tarefa atual primeiro.".to_string());
    }

    // Verificar se não há sessão ativa
    let active_session_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM active_sessions WHERE task_id = ?1",
        [task_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    if active_session_exists > 0 {
        return Err("Tarefa já tem uma sessão Pomodoro ativa".to_string());
    }

    // Buscar próxima sessão Pomodoro (a mesma lógica de start_task)
    let next_session = get_next_pomodoro_session(&conn, task_id)
        .map_err(|e| e.to_string())?;

    match next_session {
        Some(pomodoro_session) => {
            // Retomar com próxima sessão Pomodoro
            let status = start_pomodoro_session(&conn, task_id, &pomodoro_session)
                .map_err(|e| e.to_string())?;

            // Criar novo log de tempo apenas para sessões de trabalho
            if pomodoro_session.session_type == "work" {
                let now = Utc::now().to_rfc3339();
                conn.execute(
                    "INSERT INTO task_time_logs (task_id, started_at) VALUES (?1, ?2)",
                    [&task_id.to_string(), &now],
                ).map_err(|e| e.to_string())?;
                println!("▶️ Retomando tarefa {} às {} - novo log criado", task_id, now);

                // Debug: mostrar logs após retomar
                let _ = debug_task_time_logs(&conn, task_id);
            }

            println!("Tarefa {} retomada com sessão Pomodoro: {} ({})",
                task_id, pomodoro_session.session_type, status);
        }
        None => {
            // Não há mais sessões, completar tarefa
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE tasks SET status = 'completed', completed_at = ?1 WHERE id = ?2",
                [&now, &task_id.to_string()],
            ).map_err(|e| e.to_string())?;

            println!("Tarefa {} completada ao retomar - todos os ciclos Pomodoro finalizados", task_id);
        }
    }

    Ok(())
}

#[tauri::command]
async fn get_task_remaining_time(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<i64, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    // Buscar estimated_hours da tarefa
    let mut stmt = conn.prepare(
        "SELECT estimated_hours FROM tasks WHERE id = ?1"
    ).map_err(|e| e.to_string())?;

    let estimated_hours: f64 = stmt.query_row([task_id], |row| row.get(0))
        .map_err(|_| "Tarefa não encontrada".to_string())?;

    // Calcular tempo restante
    let remaining_seconds = calculate_task_remaining_time(&conn, task_id, estimated_hours)
        .map_err(|e| e.to_string())?;

    Ok(remaining_seconds)
}

#[tauri::command]
async fn delete_task(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<(), String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM tasks WHERE id = ?1",
        [&task_id.to_string()],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn increment_task_count(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<u32, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    // Buscar o valor atual do contador
    let mut stmt = conn.prepare(
        "SELECT count_value FROM tasks WHERE id = ?1"
    ).map_err(|e| e.to_string())?;

    let current_count: u32 = stmt.query_row([task_id], |row| row.get(0))
        .map_err(|_| "Tarefa não encontrada".to_string())?;

    // Incrementar o contador
    let new_count = current_count + 1;

    // Atualizar no banco
    conn.execute(
        "UPDATE tasks SET count_value = ?1 WHERE id = ?2",
        rusqlite::params![new_count, task_id],
    ).map_err(|e| e.to_string())?;

    println!("Contador da tarefa {} incrementado: {} -> {}", task_id, current_count, new_count);
    Ok(new_count)
}

#[tauri::command]
async fn update_task(
    task_id: i64,
    name: String,
    description: Option<String>,
    estimated_hours: f64,
    scheduled_date: String,
    end_date: Option<String>,
    should_count: bool,
    db_state: State<'_, DatabaseState>
) -> Result<(), String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    // Pegar o tempo trabalhado atual
    let total_worked_seconds = calculate_total_worked_seconds(&conn, task_id)
        .map_err(|e| e.to_string())?;
    let worked_hours = total_worked_seconds as f64 / 3600.0;

    conn.execute(
        "UPDATE tasks SET name = ?1, description = ?2, estimated_hours = ?3, scheduled_date = ?4, end_date = ?5, should_count = ?6 WHERE id = ?7",
        rusqlite::params![
            &name,
            description.as_deref(),
            estimated_hours,
            &scheduled_date,
            end_date.as_deref(),
            should_count,
            task_id
        ],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn reschedule_tasks(db_state: State<'_, DatabaseState>) -> Result<Vec<i64>, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    let rescheduled_tasks = reschedule_incomplete_tasks(&conn)
        .map_err(|e| e.to_string())?;

    Ok(rescheduled_tasks)
}

#[tauri::command]
async fn get_today_tasks(db_state: State<'_, DatabaseState>) -> Result<Vec<Task>, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let mut stmt = conn.prepare(
        "SELECT id, name, description, user, estimated_hours, scheduled_date, end_date, status, created_at, started_at, completed_at, should_count, count_value
         FROM tasks WHERE scheduled_date = ?1 ORDER BY created_at ASC"
    ).map_err(|e| e.to_string())?;

    let task_iter = stmt.query_map([&today], |row| {
        Ok(Task {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            description: row.get(2)?,
            user: row.get(3)?,
            estimated_hours: row.get(4)?,
            scheduled_date: row.get(5)?,
            end_date: row.get(6)?,
            status: row.get(7)?,
            created_at: row.get(8)?,
            started_at: row.get(9)?,
            completed_at: row.get(10)?,
            should_count: row.get(11)?,
            count_value: row.get(12)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut tasks = Vec::new();
    for task in task_iter {
        tasks.push(task.map_err(|e| e.to_string())?);
    }

    Ok(tasks)
}

#[tauri::command]
async fn expand_window_for_modal(window: tauri::WebviewWindow) -> Result<(), String> {
    println!("🔧 Expandindo janela para modal...");

    match window.set_size(PhysicalSize::new(1920, 800)) {
        Ok(_) => {
            println!("✓ Janela expandida para 500px");

            // Garantir que a janela esteja visível e na posição correta
            match window.set_position(LogicalPosition::new(-06.5, -1.0)) {
                Ok(_) => println!("✓ Posição da janela ajustada"),
                Err(e) => println!("✗ Erro ao ajustar posição: {}", e),
            }

            #[cfg(target_os = "linux")]
            {
                // Aplicar configurações do wmctrl para garantir
                if let Ok(output) = Command::new("wmctrl").args(["-l"]).output() {
                    let window_list = String::from_utf8_lossy(&output.stdout);
                    for line in window_list.lines() {
                        if line.contains("ClockWise") || line.contains("app") || line.contains("Clockwise") {
                            let window_id = line.split_whitespace().next().unwrap_or("");
                            let _ = Command::new("wmctrl")
                                .args(["-i", "-r", window_id, "-e", "0,0,0,1920,500"])
                                .output();
                        }
                    }
                }
            }

            Ok(())
        },
        Err(e) => {
            println!("✗ Erro ao expandir janela: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn reset_window_size(window: tauri::WebviewWindow) -> Result<(), String> {
    println!("🔧 Resetando tamanho da janela...");

    match window.set_size(PhysicalSize::new(1920, 70)) {
        Ok(_) => {
            println!("✓ Janela resetada para 55px");

            match window.set_position(LogicalPosition::new(-06.5, -1.0)) {
                Ok(_) => println!("✓ Posição da janela ajustada"),
                Err(e) => println!("✗ Erro ao ajustar posição: {}", e),
            }

            #[cfg(target_os = "linux")]
            {
                if let Ok(output) = Command::new("wmctrl").args(["-l"]).output() {
                    let window_list = String::from_utf8_lossy(&output.stdout);
                    for line in window_list.lines() {
                        if line.contains("ClockWise") || line.contains("app") || line.contains("Clockwise") {
                            let window_id = line.split_whitespace().next().unwrap_or("");
                            let _ = Command::new("wmctrl")
                                .args(["-i", "-r", window_id, "-e", "0,0,0,1920,55"])
                                .output();
                        }
                    }
                }
            }

            Ok(())
        },
        Err(e) => {
            println!("✗ Erro ao resetar janela: {}", e);
            Err(e.to_string())
        }
    }
}

#[cfg(windows)]
fn remove_window_decorations(window: tauri::WebviewWindow) {
    // Aguarda um momento para garantir que a janela foi criada
    thread::sleep(Duration::from_millis(100));

    // Obtém o HWND da janela - convertendo o ponteiro para isize
    let hwnd = HWND(window.hwnd().unwrap().0 as isize);

    unsafe {
        // Remove os estilos de borda
        let style = GetWindowLongW(hwnd, GWL_STYLE);
        let mask = (WS_CAPTION.0 | WS_THICKFRAME.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0 | WS_SYSMENU.0) as i32;
        let new_style = style & !mask;
        SetWindowLongW(hwnd, GWL_STYLE, new_style);

        // Adiciona estilos estendidos para transparência (apenas WS_EX_LAYERED, sem WS_EX_TRANSPARENT)
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        let ex_mask = WS_EX_LAYERED.0 as i32;
        let new_ex_style = ex_style | ex_mask;
        SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex_style);

        // Configura a transparência da janela
        let _ = SetLayeredWindowAttributes(
            hwnd,
            COLORREF(0), // RGB color key (0 para não usar color key)
            255, // Alpha (255 = totalmente opaco)
            LWA_ALPHA,
        );

        // Força o redraw e remove a moldura
        let _ = SetWindowPos(
            hwnd,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        );
    }
}

#[tauri::command]
async fn get_task_total_worked_time(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<i64, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;
    let total_seconds = calculate_total_worked_seconds(&conn, task_id)
        .map_err(|e| e.to_string())?;
    Ok(total_seconds)
}

#[tauri::command]
async fn get_task_by_id(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<TaskWithActiveSession, String> {
    let conn = db_state.connection.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT t.*,
                GROUP_CONCAT(CASE
                    WHEN ps.id IS NOT NULL THEN
                        json_object(
                            'id', ps.id,
                            'session_number', ps.session_number,
                            'session_type', ps.session_type,
                            'duration_seconds', ps.duration_seconds,
                            'created_at', ps.created_at,
                            'is_active', CASE WHEN acs.task_id IS NOT NULL THEN 1 ELSE 0 END,
                            'started_at', acs.started_at
                        )
                    ELSE NULL
                END) as pomodoro_sessions,
                json_object(
                    'session_type', CASE WHEN acs.task_id IS NOT NULL THEN ps_active.session_type ELSE NULL END,
                    'started_at', acs.started_at,
                    'ends_at', CASE
                        WHEN acs.started_at IS NOT NULL THEN
                            datetime(acs.started_at, '+' || ps_active.duration_seconds || ' seconds')
                        ELSE NULL
                    END,
                    'duration_seconds', ps_active.duration_seconds
                ) as active_session
        FROM tasks t
        LEFT JOIN pomodoro_sessions ps ON t.id = ps.task_id
        LEFT JOIN active_sessions acs ON t.id = acs.task_id
        LEFT JOIN pomodoro_sessions ps_active ON acs.pomodoro_id = ps_active.id
        WHERE t.id = ?
        GROUP BY t.id"
    ).map_err(|e| e.to_string())?;

    let task: TaskWithActiveSession = stmt.query_row([task_id], |row| {
        let pomodoro_sessions_str: Option<String> = row.get(13)?;
        let active_session_str: String = row.get(14)?;

        let pomodoro_sessions = if let Some(sessions_str) = pomodoro_sessions_str {
            serde_json::from_str(&format!("[{}]", sessions_str))
                .unwrap_or_else(|_| Vec::new())
        } else {
            Vec::new()
        };

        let active_session = if active_session_str.contains("null") {
            None
        } else {
            serde_json::from_str(&active_session_str).ok()
        };

        Ok(TaskWithActiveSession {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            user: row.get(3)?,
            estimated_hours: row.get(4)?,
            scheduled_date: row.get(5)?,
            end_date: row.get(6)?,
            status: row.get(7)?,
            created_at: row.get(8)?,
            started_at: row.get(9)?,
            completed_at: row.get(10)?,
            should_count: row.get(11)?,
            count_value: row.get(12)?,
            active_session,
            pomodoro_sessions,
        })
    }).map_err(|e| e.to_string())?;

    Ok(task)
}

fn main() {
    println!("Iniciando aplicação ClockWise...");


    // Tentar configurar atalho global
    println!("Criando GlobalHotKeyManager...");
    let global_hotkey_manager = match GlobalHotKeyManager::new() {
        Ok(manager) => {
            println!("✓ GlobalHotKeyManager criado com sucesso");
            Some(Arc::new(manager))
        }
        Err(e) => {
            eprintln!("✗ Erro ao criar GlobalHotKeyManager: {}", e);
            None
        }
    };

    // Tentar registrar o atalho global
    let hotkey_registered = if let Some(ref manager) = global_hotkey_manager {
        let hotkey = HotKey::new(Some(Modifiers::ALT), Code::KeyC);
        println!("Registrando atalho Alt+C (ID: {})...", hotkey.id());

        match manager.register(hotkey) {
            Ok(_) => {
                println!("✓ Atalho global Alt+C registrado com sucesso!");
                true
            }
            Err(e) => {
                eprintln!("✗ Erro ao registrar atalho global: {}", e);
                false
            }
        }
    } else {
        false
    };

    let db = init_database().expect("Failed to initialize database");
    let db_state = DatabaseState {
        connection: Arc::new(Mutex::new(db)),
    };

    tauri::Builder::default()
        .manage(db_state)
        .invoke_handler(tauri::generate_handler![
            load_tasks,
            add_task,
            start_task,
            complete_task,
            pause_task,
            resume_task,
            delete_task,
            update_task,
            get_task_remaining_time,
            get_task_total_worked_time,
            get_task_by_id,
            increment_task_count,
            reschedule_tasks,
            get_today_tasks,
            check_pomodoro_sessions,
            load_tasks_with_sessions,
            toggle_collapse,
            get_collapsed_state,
            test_hotkey_manually,
            get_system_volume,
            set_system_volume,
            get_system_mute_status,
            toggle_system_mute,
            expand_window_for_modal,
            reset_window_size,
        ])
        .setup(move |app| {
            let handle = app.handle();
            let window = handle.get_webview_window("main").unwrap();

            #[cfg(windows)]
            remove_window_decorations(window.clone());

            // Configurar janela
            window.set_decorations(false)?;
            window.set_size(PhysicalSize::new(1920, 55))?;
            window.set_always_on_top(true)?;

            // Posicionar a janela
            window.set_position(PhysicalPosition::new(-7, -2))?;

                        // Thread para remanejamento automático de tarefas
            let app_handle = handle.clone();
            thread::spawn(move || {
                let mut last_check_day = chrono::Local::now().date_naive();

                loop {
                    thread::sleep(Duration::from_secs(3600)); // Verificar a cada hora

                    let current_day = chrono::Local::now().date_naive();

                    // Se mudou de dia, executar remanejamento
                    if current_day != last_check_day {
                        println!("🌅 Novo dia detectado: {} -> {}", last_check_day, current_day);

                        if let Some(db_state) = app_handle.try_state::<DatabaseState>() {
                            if let Ok(conn) = db_state.connection.lock() {
                                match reschedule_incomplete_tasks(&conn) {
                                    Ok(rescheduled) => {
                                        if !rescheduled.is_empty() {
                                            println!("🔄 Remanejamento automático: {} tarefas movidas", rescheduled.len());
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("❌ Erro no remanejamento automático: {}", e);
                                    }
                                }
                            }
                        }

                        last_check_day = current_day;
                    }
                }
            });

            // Thread para monitorar mudanças de volume do sistema
            let window_for_volume = window.clone();
            thread::spawn(move || {
                let mut last_volume = 50;
                let mut last_mute = false;

                loop {
                    thread::sleep(Duration::from_millis(1000)); // Verificar a cada segundo

                    // Verificar volume atual
                    if let Ok(output) = Command::new("pactl")
                        .args(["get-sink-volume", "@DEFAULT_SINK@"])
                        .output()
                    {
                        let output_str = String::from_utf8_lossy(&output.stdout);
                        for line in output_str.lines() {
                            if let Some(percent_pos) = line.find('%') {
                                let before_percent = &line[..percent_pos];
                                if let Some(space_pos) = before_percent.rfind(' ') {
                                    let volume_str = &before_percent[space_pos + 1..];
                                    if let Ok(current_volume) = volume_str.parse::<i32>() {
                                        if current_volume != last_volume {
                                            last_volume = current_volume;
                                            let _ = window_for_volume.emit("volume-changed", current_volume);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Verificar status de mute
                    if let Ok(output) = Command::new("pactl")
                        .args(["get-sink-mute", "@DEFAULT_SINK@"])
                        .output()
                    {
                        let output_str = String::from_utf8_lossy(&output.stdout);
                        let current_mute = output_str.trim() == "yes";
                        if current_mute != last_mute {
                            last_mute = current_mute;
                            let _ = window_for_volume.emit("mute-changed", current_mute);
                        }
                    }
                }
            });

            // Se conseguiu registrar o atalho global, configurar o listener
            if hotkey_registered {
                let window_clone = window.clone();

                thread::spawn(move || {
                    let receiver = GlobalHotKeyEvent::receiver();
                    println!("🎯 Thread de atalho global iniciada, aguardando eventos...");
                    println!("   Pressione Alt+C para testar o atalho global");

                    loop {
                        match receiver.try_recv() {
                            Ok(event) => {
                                println!("📨 Evento recebido: ID={}", event.id);

                                // Verificar debounce
                                if !should_process_hotkey() {
                                    continue;
                                }

                                println!("🎉 Alt+C detectado globalmente!");

                                // Alternar estado
                                let new_state = if let Ok(state) = COLLAPSED_STATE.lock() {
                                    let current = *state;
                                    println!("🔧 Estado atual: {}, novo estado será: {}", current, !current);
                                    !current
                                } else {
                                    println!("🔧 Erro ao ler estado, usando false");
                                    false
                                };

                                // Emitir evento para o frontend
                                match window_clone.emit("global-hotkey-pressed", new_state) {
                                    Ok(_) => println!("✓ Evento emitido para o frontend"),
                                    Err(e) => eprintln!("✗ Erro ao emitir evento: {}", e),
                                }

                                // Aplicar as mudanças na janela diretamente
                                let window_for_toggle = window_clone.clone();
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = toggle_collapse(window_for_toggle, new_state).await {
                                        eprintln!("✗ Erro ao aplicar toggle_collapse: {}", e);
                                    }
                                });
                            }
                            Err(_) => {
                                // Não há eventos, continuar silenciosamente
                            }
                        }
                        thread::sleep(Duration::from_millis(50));
                    }
                });
            } else {
                println!("⚠️  Atalho global não foi registrado. Use o botão de teste manual na interface.");
            }

            #[cfg(target_os = "linux")]
            {
                // Esperar um pouco para a janela ser criada
                thread::sleep(Duration::from_millis(500));

                // Tentar encontrar e configurar a janela usando wmctrl
                if let Ok(output) = Command::new("wmctrl")
                    .args(["-l"])
                    .output()
                {
                    let window_list = String::from_utf8_lossy(&output.stdout);

                    // Procurar pela nossa janela
                    for line in window_list.lines() {
                        if line.contains("ClockWise") || line.contains("app") {
                            let window_id = line.split_whitespace().next().unwrap_or("");

                            // Configurar a janela como dock
                            let _ = Command::new("wmctrl")
                                .args(["-i", "-r", window_id, "-b", "add,above"])
                                .output();

                            // Forçar o tamanho
                            let _ = Command::new("wmctrl")
                                .args(["-i", "-r", window_id, "-e", "0,0,0,1920,55"])
                                .output();

                            println!("Rust: Configurações wmctrl aplicadas");
                            break;
                        }
                    }
                } else {
                    println!("Rust: wmctrl não encontrado. Por favor, instale com: sudo apt install wmctrl");
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::test::MockWebviewWindow;

    // Função auxiliar para inicializar schema do banco de dados de teste
    fn init_database_schema(conn: &Connection) -> SqliteResult<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                user TEXT NOT NULL,
                estimated_hours REAL NOT NULL,
                scheduled_date TEXT NOT NULL,
                end_date TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT,
                should_count BOOLEAN NOT NULL,
                count_value INTEGER
            )",
            [],
        )?;

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

        Ok(())
    }

    // Teste para expand_window_for_modal
    #[tokio::test]
    async fn test_expand_window_for_modal() {
        let mock_window = MockWebviewWindow::new();

        // Teste expandir janela
        let result = expand_window_for_modal(mock_window.clone()).await;
        assert!(result.is_ok(), "Deveria expandir a janela com sucesso");

        // Verificar se o tamanho foi definido corretamente
        let size = mock_window.size().unwrap();
        assert_eq!(size.width, 1920);
        assert_eq!(size.height, 500);

        // Verificar posição
        let position = mock_window.position().unwrap();
        assert_eq!(position.x, 0.0);
        assert_eq!(position.y, 0.0);
    }

    // Teste para reset_window_size
    #[tokio::test]
    async fn test_reset_window_size() {
        let mock_window = MockWebviewWindow::new();

        // Teste resetar tamanho
        let result = reset_window_size(mock_window.clone()).await;
        assert!(result.is_ok(), "Deveria resetar a janela com sucesso");

        // Verificar se o tamanho foi resetado corretamente
        let size = mock_window.size().unwrap();
        assert_eq!(size.width, 1920);
        assert_eq!(size.height, 55);

        // Verificar posição
        let position = mock_window.position().unwrap();
        assert_eq!(position.x, 0.0);
        assert_eq!(position.y, 0.0);
    }

    // Testes para as funções de tarefa
    #[tokio::test]
    async fn test_task_lifecycle() {
        // Criar conexão de teste
        let conn = Connection::open_in_memory().unwrap();
        init_database_schema(&conn).unwrap();

        let db_state = DatabaseState {
            connection: Arc::new(Mutex::new(conn)),
        };

        let task = add_task(
            "Teste".to_string(),
            Some("Descrição do teste".to_string()),
            "Usuário".to_string(),
            2.0,
            "2024-03-14".to_string(),
            Some("2024-03-15".to_string()),
            true,
            100,
            State::new(db_state.clone())
        ).await;
        assert!(task.is_ok(), "Deveria criar tarefa com sucesso");
        let task = task.unwrap();

        // Teste iniciar tarefa
        let start_result = start_task(
            task.id.unwrap(),
            State::new(db_state.clone())
        ).await;
        assert!(start_result.is_ok(), "Deveria iniciar tarefa com sucesso");

        // Verificar se apenas uma tarefa pode estar ativa
        let another_task = add_task(
            "Outra".to_string(),
            Some("Descrição da outra tarefa".to_string()),
            "Usuário".to_string(),
            1.0,
            "2024-03-14".to_string(),
            None,
            false,
            0,
            State::new(db_state.clone())
        ).await.unwrap();

        let start_another = start_task(
            another_task.id.unwrap(),
            State::new(db_state.clone())
        ).await;
        assert!(start_another.is_err(), "Não deveria permitir iniciar outra tarefa");

        // Teste completar tarefa
        let complete_result = complete_task(
            task.id.unwrap(),
            State::new(db_state.clone())
        ).await;
        assert!(complete_result.is_ok(), "Deveria completar tarefa com sucesso");

        // Teste deletar tarefa
        let delete_result = delete_task(
            task.id.unwrap(),
            State::new(db_state.clone())
        ).await;
        assert!(delete_result.is_ok(), "Deveria deletar tarefa com sucesso");
    }

    // Teste para get_today_tasks
    #[tokio::test]
    async fn test_get_today_tasks() {
        // Criar conexão de teste
        let conn = Connection::open_in_memory().unwrap();
        init_database_schema(&conn).unwrap();

        let db_state = DatabaseState {
            connection: Arc::new(Mutex::new(conn)),
        };

        // Adicionar algumas tarefas
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let yesterday = (chrono::Local::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d").to_string();

        // Tarefa de hoje
        add_task(
            "Hoje".to_string(),
            Some("Tarefa de hoje".to_string()),
            "Usuário".to_string(),
            1.0,
            today.clone(),
            None,
            true,
            50,
            State::new(db_state.clone())
        ).await.unwrap();

        // Tarefa de ontem
        add_task(
            "Ontem".to_string(),
            Some("Tarefa de ontem".to_string()),
            "Usuário".to_string(),
            1.0,
            yesterday,
            None,
            false,
            0,
            State::new(db_state.clone())
        ).await.unwrap();

        // Buscar tarefas de hoje
        let today_tasks = get_today_tasks(State::new(db_state.clone())).await.unwrap();

        assert_eq!(today_tasks.len(), 1, "Deveria retornar apenas uma tarefa de hoje");
        assert_eq!(today_tasks[0].scheduled_date, today);
    }
}
