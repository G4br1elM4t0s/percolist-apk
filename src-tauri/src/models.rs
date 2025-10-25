use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthData {
    pub token: String,
    pub session_id: String,
    pub user_data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthCallback {
    pub token: String,
    pub session_id: String,
    pub code: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub user: String,
    pub estimated_hours: f64,
    pub scheduled_date: String,
    pub end_date: Option<String>,
    pub status: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub should_count: bool,
    pub count_value: u32,
    pub pomodoro_cycles: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskTimeLog {
    pub id: Option<i64>,
    pub task_id: i64,
    pub started_at: String,
    pub ended_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PomodoroSession {
    pub id: Option<i64>,
    pub task_id: i64,
    pub session_number: i32,
    pub session_type: String, // "work" or "break"
    pub duration_seconds: i32,
    pub remaining_seconds: i32,
    pub status: String, // "pending", "running", "paused", "completed"
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActiveSession {
    pub task_id: i64,
    pub pomodoro_id: i64,
    pub started_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskWithActiveSession {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub user: String,
    pub estimated_hours: f64,
    pub scheduled_date: String,
    pub end_date: Option<String>,
    pub status: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub should_count: bool,
    pub count_value: u32,
    pub pomodoro_cycles: u32,
    pub active_session: Option<ActiveSessionInfo>,
    pub pomodoro_sessions: Vec<PomodoroSessionInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PomodoroSessionInfo {
    pub id: Option<i64>,
    pub session_number: i32,
    pub session_type: String,
    pub duration_seconds: i32,
    pub created_at: String,
    pub is_active: bool,
    pub started_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActiveSessionInfo {
    pub session_type: String,
    pub started_at: String,
    pub ends_at: String,
    pub duration_seconds: i32,
}
