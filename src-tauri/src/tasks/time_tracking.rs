use crate::database::DatabaseState;
use rusqlite::Connection;
use tauri::State;
use chrono::Utc;

pub fn calculate_total_worked_seconds(conn: &Connection, task_id: i64) -> Result<i64, rusqlite::Error> {
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

pub fn calculate_task_remaining_time(conn: &Connection, task_id: i64, estimated_hours: f64) -> Result<i64, rusqlite::Error> {
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

pub fn debug_task_time_logs(conn: &Connection, task_id: i64) -> Result<(), rusqlite::Error> {
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

#[tauri::command]
pub async fn get_task_remaining_time(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<i64, String> {
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
pub async fn get_task_total_worked_time(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<i64, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;
    let total_seconds = calculate_total_worked_seconds(&conn, task_id)
        .map_err(|e| e.to_string())?;
    Ok(total_seconds)
}
