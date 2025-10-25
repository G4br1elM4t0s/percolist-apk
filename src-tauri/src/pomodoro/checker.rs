use crate::models::*;
use crate::database::DatabaseState;
use crate::pomodoro::sessions::{get_next_pomodoro_session, start_pomodoro_session};
use rusqlite::Connection;
use tauri::State;
use chrono::Utc;

pub fn check_and_advance_pomodoro_sessions(conn: &Connection) -> Result<Vec<i64>, rusqlite::Error> {
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

pub async fn load_tasks_with_sessions(db_state: State<'_, DatabaseState>) -> Result<Vec<TaskWithActiveSession>, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.description, t.user, t.estimated_hours, t.scheduled_date, t.end_date, t.status,
                t.created_at, t.started_at, t.completed_at, t.should_count, t.count_value, t.pomodoro_cycles,
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
        let session_started_at: Option<String> = row.get(14)?;
        let session_type: Option<String> = row.get(15)?;
        let duration_seconds: Option<i32> = row.get(16)?;

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
            pomodoro_cycles: row.get(13)?,
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
pub async fn check_pomodoro_sessions(db_state: State<'_, DatabaseState>) -> Result<Vec<i64>, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    let advanced_tasks = check_and_advance_pomodoro_sessions(&conn)
        .map_err(|e| e.to_string())?;

    Ok(advanced_tasks)
}

#[tauri::command]
pub async fn load_tasks_with_sessions_command(db_state: State<'_, DatabaseState>) -> Result<Vec<TaskWithActiveSession>, String> {
    load_tasks_with_sessions(db_state).await
}
