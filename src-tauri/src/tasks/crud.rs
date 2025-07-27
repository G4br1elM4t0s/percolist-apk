use crate::models::*;
use crate::database::DatabaseState;
use tauri::State;
use chrono::Utc;

#[tauri::command]
pub async fn load_tasks(db_state: State<'_, DatabaseState>) -> Result<Vec<Task>, String> {
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
pub async fn add_task(
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
    crate::pomodoro::create_pomodoro_cycles(&conn, id).map_err(|e| e.to_string())?;
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
pub async fn update_task(
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
pub async fn delete_task(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<(), String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM tasks WHERE id = ?1",
        [&task_id.to_string()],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_task_by_id(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<TaskWithActiveSession, String> {
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

#[tauri::command]
pub async fn increment_task_count(task_id: i64, db_state: State<'_, DatabaseState>) -> Result<u32, String> {
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
pub async fn get_today_tasks(db_state: State<'_, DatabaseState>) -> Result<Vec<Task>, String> {
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
