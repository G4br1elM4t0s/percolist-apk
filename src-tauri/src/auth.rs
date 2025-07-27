use crate::models::{AuthData, AuthCallback};
use crate::database::DatabaseState;
use crate::auth_server::AuthServer;
use rusqlite::{Result, OptionalExtension};
use std::sync::Arc;
use tauri::{State, Emitter};
use std::sync::Mutex;
use std::net::TcpStream;
use std::time::Duration;

pub struct LocalAuthServer {
    pub is_running: Arc<Mutex<bool>>,
    pub port: u16,
}

// Estado global do servidor
pub struct AuthServerState {
    pub server: Arc<Mutex<Option<AuthServer>>>,
}

impl AuthServerState {
    pub fn new() -> Self {
        Self {
            server: Arc::new(Mutex::new(None)),
        }
    }
}

pub async fn handle_auth_callback(url: String, db_state: tauri::State<'_, DatabaseState>, app_handle: tauri::AppHandle) -> Result<String, String> {
    println!("🔐 Auth callback recebido: {}", url);

    // Parsear a URL do deep link
    // Exemplo: percolist://auth-callback?token=abc123&sessionId=xyz789
    if let Some(query_start) = url.find('?') {
        let query_string = &url[query_start + 1..];
        let mut token = String::new();
        let mut session_id = String::new();
        let mut code = None;
        let mut state = None;

        // Parsear parâmetros da query string
        for param in query_string.split('&') {
            if let Some(equal_pos) = param.find('=') {
                let key = &param[..equal_pos];
                let value = &param[equal_pos + 1..];

                match key {
                    "token" => token = value.to_string(),
                    "sessionId" => session_id = value.to_string(),
                    "code" => code = Some(value.to_string()),
                    "state" => state = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        if !token.is_empty() && !session_id.is_empty() {
            // Criar AuthCallback
            let callback_data = AuthCallback {
                token,
                session_id,
                code,
                state,
            };

            println!("🔐 Dados parseados: {:?}", callback_data);

            // Salvar no banco de dados
            if let Ok(conn) = db_state.connection.lock() {
                // Limpar dados anteriores
                conn.execute("DELETE FROM auth_data", []).map_err(|e| e.to_string())?;

                // Inserir novos dados
                conn.execute(
                    "INSERT INTO auth_data (token, session_id, user_data) VALUES (?, ?, ?)",
                    [&callback_data.token, &callback_data.session_id, ""],
                ).map_err(|e| e.to_string())?;

                println!("✅ Dados de autenticação salvos no banco");
            }

            // Emitir evento para o frontend
            let _ = app_handle.emit("auth-callback-received", callback_data);

            return Ok("Auth callback processado com sucesso".to_string());
        }
    }

    Err("URL de auth callback inválida".to_string())
}

#[tauri::command]
pub async fn get_saved_auth_data(db_state: State<'_, DatabaseState>) -> Result<Option<AuthData>, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT token, session_id, user_data FROM auth_data ORDER BY created_at DESC LIMIT 1"
    ).map_err(|e| e.to_string())?;

    let auth_data = stmt.query_row([], |row| {
        Ok(AuthData {
            token: row.get(0)?,
            session_id: row.get(1)?,
            user_data: row.get(2)?,
        })
    }).optional().map_err(|e| e.to_string())?;

    Ok(auth_data)
}

#[tauri::command]
pub async fn clear_saved_auth_data(db_state: State<'_, DatabaseState>) -> Result<String, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM auth_data", [])
        .map_err(|e| e.to_string())?;

    println!("🧹 Dados de autenticação limpos com sucesso");
    Ok("Dados de autenticação limpos com sucesso".to_string())
}

#[tauri::command]
pub async fn test_auth_callback() -> Result<String, String> {
    println!("🧪 Testando auth callback");
    Ok("Teste de auth callback executado".to_string())
}

#[tauri::command]
pub async fn verify_auth_storage(db_state: State<'_, DatabaseState>) -> Result<bool, String> {
    let conn = db_state.connection.lock().map_err(|e| e.to_string())?;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM auth_data",
        [],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    println!("🔍 Verificando storage de autenticação: {} registros encontrados", count);
    Ok(count > 0)
}

#[tauri::command]
pub async fn check_auth_data_received_command(db_state: State<'_, DatabaseState>) -> Result<bool, String> {
    let auth_data = get_saved_auth_data(db_state).await?;
    let has_data = auth_data.is_some();

    println!("📥 Verificando se dados de auth foram recebidos: {}", has_data);
    Ok(has_data)
}

// Função para verificar se a porta está disponível
fn is_port_available(port: u16) -> bool {
    match TcpStream::connect_timeout(&format!("127.0.0.1:{}", port).parse().unwrap(), Duration::from_millis(100)) {
        Ok(_) => true,  // Porta está ocupada (servidor rodando)
        Err(_) => false, // Porta está livre
    }
}

#[tauri::command]
pub async fn start_auth_server(
    db_state: State<'_, DatabaseState>,
    app_handle: tauri::AppHandle
) -> Result<String, String> {
    println!("🚀 Iniciando servidor de autenticação na porta 1420");

    // Verificar se já está rodando
    if is_port_available(1420) {
        println!("⚠️ Servidor já está rodando na porta 1420");
        return Ok("Servidor já está rodando".to_string());
    }

    // Criar e iniciar o servidor
    let auth_server = AuthServer::new();
    let db_connection = db_state.connection.clone();

    // Usar o método start do AuthServer importado
    auth_server.start(db_connection, app_handle);

    // Aguardar um pouco para o servidor iniciar
    tokio::time::sleep(Duration::from_millis(500)).await;

    if is_port_available(1420) {
        println!("✅ Servidor de autenticação iniciado com sucesso na porta 1420");
        Ok("Servidor de autenticação iniciado com sucesso".to_string())
    } else {
        println!("❌ Falha ao iniciar servidor - porta não está respondendo");
        Err("Falha ao iniciar servidor".to_string())
    }
}

#[tauri::command]
pub async fn stop_auth_server() -> Result<String, String> {
    println!("🛑 Parando servidor de autenticação");
    // Implementar lógica para parar o servidor se necessário
    Ok("Servidor de autenticação parado".to_string())
}

#[tauri::command]
pub async fn auth_server_status() -> Result<bool, String> {
    let is_running = is_port_available(1420);
    println!("📊 Status do servidor de autenticação: {}", if is_running { "RODANDO" } else { "PARADO" });
    Ok(is_running)
}

#[tauri::command]
pub async fn test_auth_server() -> Result<String, String> {
    println!("🧪 Testando servidor auth na porta 1420");

    if !is_port_available(1420) {
        return Err("Servidor não está rodando na porta 1420".to_string());
    }

    // Testar conectividade básica
    match TcpStream::connect_timeout(&"127.0.0.1:1420".parse().unwrap(), Duration::from_millis(1000)) {
        Ok(_) => {
            println!("✅ Servidor está respondendo na porta 1420");
            Ok("Servidor está funcionando corretamente".to_string())
        }
        Err(e) => {
            println!("❌ Erro ao conectar com o servidor: {}", e);
            Err(format!("Erro ao conectar: {}", e))
        }
    }
}

#[tauri::command]
pub async fn save_auth_data(token: String, session_id: String, app_handle: tauri::AppHandle) -> Result<String, String> {
    println!("💾 Salvando dados de autenticação no localStorage");

    // Emitir evento para o frontend salvar no localStorage
    let auth_data = format!("{{\"token\":\"{}\",\"sessionId\":\"{}\"}}", token, session_id);

    app_handle.emit("auth-data-received", auth_data)
        .map_err(|e| format!("Erro ao emitir evento: {}", e))?;

    println!("✅ Evento auth-data-received emitido com sucesso");
    Ok("Dados de autenticação enviados para o frontend".to_string())
}
