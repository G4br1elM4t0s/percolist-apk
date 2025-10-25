use crate::models::*;
use crate::database::DatabaseState;
use crate::tasks::time_tracking::debug_task_time_logs;
use rusqlite::{Connection, OptionalExtension, Result as SqliteResult, params};
use tauri::State;
use chrono::Utc;

pub fn create_pomodoro_cycles(conn: &Connection, task_id: i64, estimated_hours: f64, cycles: u32) -> SqliteResult<()> {
    let total_minutes = (estimated_hours * 60.0).round() as i32;
    let work_minutes = total_minutes / cycles as i32;
    let mini_break_minutes = 5;
    let long_break_minutes = 15;

    let mut session_number = 1;
    let now = Utc::now().to_rfc3339();

    for cycle in 1..=cycles {
        conn.execute(
            "INSERT INTO pomodoro_sessions (task_id, session_number, session_type, duration_seconds, remaining_seconds, status, created_at)
             VALUES (?1, ?2, 'work', ?3, ?3, 'pending', ?4)",
            params![task_id, session_number, work_minutes * 60, &now],
        )?;
        session_number += 1;

        if cycle < cycles {
            conn.execute(
                "INSERT INTO pomodoro_sessions (task_id, session_number, session_type, duration_seconds, remaining_seconds, status, created_at)
                 VALUES (?1, ?2, 'mini_break', ?3, ?3, 'pending', ?4)",
                params![task_id, session_number, mini_break_minutes * 60, &now],
            )?;
            session_number += 1;
        }
    }

    conn.execute(
        "INSERT INTO pomodoro_sessions (task_id, session_number, session_type, duration_seconds, remaining_seconds, status, created_at)
         VALUES (?1, ?2, 'long_break', ?3, ?3, 'pending', ?4)",
        params![task_id, session_number, long_break_minutes * 60, &now],
    )?;

    Ok(())
}

pub fn get_next_pomodoro_session(conn: &Connection, task_id: i64) -> Result<Option<PomodoroSession>, rusqlite::Error> {
    // Verificar se já existem sessões para esta tarefa
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pomodoro_sessions WHERE task_id = ?1",
        [task_id],
        |row| row.get(0),
    )?;

    // Se não existem sessões, criar os ciclos
    if count == 0 {
        // Buscar estimated_hours da tarefa
        let estimated_hours: f64 = conn.query_row(
            "SELECT estimated_hours FROM tasks WHERE id = ?1",
            [task_id],
            |row| row.get(0),
        )?;

        // Criar 4 ciclos por padrão
        create_pomodoro_cycles(conn, task_id, estimated_hours, 4)?;
    }

    // Buscar a próxima sessão disponível (menor session_number que não está em uso)
    let mut stmt = conn.prepare(
        "SELECT ps.id, ps.task_id, ps.session_number, ps.session_type, ps.duration_seconds, ps.remaining_seconds, ps.status, ps.created_at
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
            remaining_seconds: row.get(5)?,
            status: row.get(6)?,
            created_at: row.get(7)?,
        })
    }).optional()?;

    Ok(session_opt)
}

pub fn start_pomodoro_session(conn: &Connection, task_id: i64, pomodoro_session: &PomodoroSession) -> Result<String, rusqlite::Error> {
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

    // Atualizar status da sessão Pomodoro para "running"
    conn.execute(
        "UPDATE pomodoro_sessions SET status = 'running' WHERE id = ?1",
        [pomodoro_session.id.unwrap()],
    )?;

    Ok(status.to_string())
}

#[tauri::command]
pub async fn start_task(task_id: i64, stop_and_start: Option<bool>, db_state: State<'_, DatabaseState>) -> Result<(), String> {
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

            // Atualizar status para 'in_progress' para que get_active_task_id funcione
            conn.execute(
                "UPDATE tasks SET status = 'in_progress' WHERE id = ?1",
                [&task_id.to_string()],
            ).map_err(|e| e.to_string())?;

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
pub async fn complete_task(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<(), String> {
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
pub async fn pause_task(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<(), String> {
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

    // 🔧 PROTEÇÃO CRÍTICA: Verificar se o tempo foi atualizado recentemente
    // Se não foi, usar o tempo calculado atual para evitar perda de precisão
    let current_remaining: Option<i64> = conn.query_row(
        "SELECT remaining_seconds FROM tasks WHERE id = ?1",
        [task_id],
        |row| row.get(0),
    ).ok(); // Usar .ok() para não falhar se a coluna não existir

    if let Some(remaining) = current_remaining {
        println!("⏸️ Pausando tarefa {} com tempo atualizado: {}s", task_id, remaining);
    } else {
        println!("⏸️ Pausando tarefa {} - tempo não atualizado, usando cálculo padrão", task_id);
    }

    // Finalizar log de tempo ANTES de remover sessão ativa (para evitar conflito com check_and_advance)
    let rows_updated = conn.execute(
        "UPDATE task_time_logs SET ended_at = ?1 WHERE task_id = ?2 AND ended_at IS NULL",
        [&now, &task_id.to_string()],
    ).map_err(|e| e.to_string())?;

    println!("⏸️ Pausando tarefa {} às {} - {} logs finalizados", task_id, now, rows_updated);

    // Debug: mostrar logs após pausar
    let _ = debug_task_time_logs(&conn, task_id);

    // Buscar o ID da sessão Pomodoro ativa para atualizar seu status
    let pomodoro_id: Option<i64> = conn.query_row(
        "SELECT pomodoro_id FROM active_sessions WHERE task_id = ?1",
        [task_id],
        |row| row.get(0),
    ).ok();

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

    // Atualizar status da sessão Pomodoro para "paused"
    if let Some(pomodoro_id) = pomodoro_id {
        conn.execute(
            "UPDATE pomodoro_sessions SET status = 'paused' WHERE id = ?1",
            [pomodoro_id],
        ).map_err(|e| e.to_string())?;
        println!("⏸️ Sessão Pomodoro {} pausada", pomodoro_id);
    }

    println!("Tarefa {} pausada - sessão Pomodoro interrompida", task_id);
    Ok(())
}

#[tauri::command]
pub async fn resume_task(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<(), String> {
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

            // Atualizar status da tarefa para 'in_progress' para que get_active_task_id funcione
            conn.execute(
                "UPDATE tasks SET status = 'in_progress' WHERE id = ?1",
                [&task_id.to_string()],
            ).map_err(|e| e.to_string())?;

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
