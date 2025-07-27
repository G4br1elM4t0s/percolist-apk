use crate::database::DatabaseState;
use crate::tasks::time_tracking::calculate_total_worked_seconds;
use rusqlite::Connection;
use tauri::State;
use chrono::{self, Datelike};

fn get_next_business_day(date: chrono::NaiveDate) -> chrono::NaiveDate {
    let mut next_day = date + chrono::Duration::days(1);

    // Pular fins de semana (sábado = 6, domingo = 0)
    while next_day.weekday().num_days_from_monday() >= 5 {
        next_day = next_day + chrono::Duration::days(1);
    }

    next_day
}

pub fn reschedule_incomplete_tasks(conn: &Connection) -> Result<Vec<i64>, rusqlite::Error> {
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

#[tauri::command]
pub async fn reschedule_tasks(db_state: State<'_, DatabaseState>) -> Result<Vec<i64>, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    let rescheduled_tasks = reschedule_incomplete_tasks(&conn)
        .map_err(|e| e.to_string())?;

    Ok(rescheduled_tasks)
}
