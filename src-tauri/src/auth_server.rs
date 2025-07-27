use std::sync::Arc;
use std::sync::Mutex;
use std::net::TcpListener;
use std::io::{Read, Write};
use std::thread;
use tauri::{AppHandle, Emitter};

pub struct AuthServer {
    pub is_running: Arc<Mutex<bool>>,
    pub port: u16,
}

impl AuthServer {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(Mutex::new(false)),
            port: 1420,
        }
    }

    pub fn start(&self, db_state: Arc<Mutex<rusqlite::Connection>>, app_handle: tauri::AppHandle) {
        let is_running = self.is_running.clone();
        let port = self.port;

        thread::spawn(move || {
            println!("🚀 Iniciando servidor de autenticação na porta {}", port);

            // Marcar como rodando
            if let Ok(mut running) = is_running.lock() {
                *running = true;
            }

            // Bind do listener
            let listener = match TcpListener::bind(format!("127.0.0.1:{}", port)) {
                Ok(listener) => {
                    println!("✅ Servidor iniciado com sucesso na porta {}", port);
                    listener
                }
                Err(e) => {
                    eprintln!("❌ Erro ao iniciar servidor: {}", e);
                    return;
                }
            };

            // Aceitar conexões
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        // Configurar timeout na conexão
                        if let Err(e) = stream.set_read_timeout(Some(std::time::Duration::from_secs(10))) {
                            eprintln!("⚠️ Erro ao configurar timeout de leitura: {}", e);
                        }
                        if let Err(e) = stream.set_write_timeout(Some(std::time::Duration::from_secs(10))) {
                            eprintln!("⚠️ Erro ao configurar timeout de escrita: {}", e);
                        }

                        let db_state_clone = db_state.clone();
                        let app_handle_clone = app_handle.clone();

                        thread::spawn(move || {
                            handle_connection(&mut stream, &db_state_clone, &app_handle_clone);
                        });
                    }
                    Err(e) => {
                        eprintln!("❌ Erro ao aceitar conexão: {}", e);
                    }
                }
            }

            // Marcar como parado
            if let Ok(mut running) = is_running.lock() {
                *running = false;
            }
        });
    }
}

fn handle_connection(stream: &mut std::net::TcpStream, db_state: &Arc<Mutex<rusqlite::Connection>>, app_handle: &tauri::AppHandle) {
    let mut buffer = [0; 2048];

    match stream.read(&mut buffer) {
        Ok(n) if n > 0 => {
            let request = String::from_utf8_lossy(&buffer[..n]);
            println!("📥 Request recebida: {}", request.lines().next().unwrap_or(""));

            let response = handle_request(&request, db_state, app_handle);

            // Enviar resposta em partes para garantir que tudo seja enviado
            let response_bytes = response.as_bytes();
            let mut total_sent = 0;

            while total_sent < response_bytes.len() {
                match stream.write(&response_bytes[total_sent..]) {
                    Ok(n) => {
                        total_sent += n;
                        if total_sent >= response_bytes.len() {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Erro ao enviar resposta: {}", e);
                        return;
                    }
                }
            }

            // Garantir que tudo seja enviado
            if let Err(e) = stream.flush() {
                eprintln!("❌ Erro ao flushar stream: {}", e);
            }

            // Aguardar um pouco antes de fechar a conexão
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        _ => {
            eprintln!("❌ Erro ao ler request");
        }
    }
}

fn handle_request(request: &str, db_state: &Arc<Mutex<rusqlite::Connection>>, app_handle: &tauri::AppHandle) -> String {
    let lines: Vec<&str> = request.lines().collect();

    if lines.is_empty() {
        return "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nInvalid request".to_string();
    }

    let first_line = lines[0];
    println!("🔍 Method: {}", first_line);

    // Parse da primeira linha: "GET /path HTTP/1.1"
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nInvalid request format".to_string();
    }

    let method = parts[0];
    let path = parts[1];

    println!("🔍 Method: {}, Path: {}", method, path);

    match (method, path) {
        ("GET", path) if path.starts_with("/auth-callback") || path.starts_with("/auth/callback") => {
            println!("✅ Rota auth-callback encontrada (com parâmetros)");

            // Extrair query string
            let query_string = if let Some(pos) = path.find('?') {
                &path[pos + 1..]
            } else {
                ""
            };

            println!("🔍 Query string: {}", query_string);
            process_auth_callback(query_string, db_state, app_handle)
        }
        _ => {
            let error_html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>404 - Not Found</title>
            </head>
            <body style="font-family: Arial, sans-serif; text-align: center; padding: 50px; background: #667eea; color: white;">
                <h1>404 - Página não encontrada</h1>
                <p>Use: /auth-callback?token=...&sessionId=...</p>
            </body>
            </html>
            "#;

            format!("HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n{}", error_html)
        }
    }
}

fn process_auth_callback(query_string: &str, _db_state: &Arc<Mutex<rusqlite::Connection>>, app_handle: &tauri::AppHandle) -> String {
    println!("🔐 Processando auth callback: {}", query_string);

    let mut token = String::new();
    let mut session_id = String::new();

    // Parse dos parâmetros
    for param in query_string.split('&') {
        if let Some(equal_pos) = param.find('=') {
            let key = &param[..equal_pos];
            let value = &param[equal_pos + 1..];

            match key {
                "token" => token = value.to_string(),
                "sessionId" => session_id = value.to_string(),
                _ => {}
            }
        }
    }

    if token.is_empty() || session_id.is_empty() {
        println!("❌ Token ou Session ID ausentes");

        let error_html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Erro - Dados Ausentes</title>
        </head>
        <body style="font-family: Arial, sans-serif; text-align: center; padding: 50px; background: #ff6b6b; color: white;">
            <h1>❌ Erro na Autenticação</h1>
            <p>Token ou Session ID ausentes na URL.</p>
            <p>URL deve ser: <code>localhost:1420/auth-callback?token=...&sessionId=...</code></p>
        </body>
        </html>
        "#;

        return format!("HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n{}", error_html);
    }

    println!("✅ Token: {}, SessionId: {}", token, session_id);
    println!("✅ Dados recebidos - Token: {}, SessionId: {}", token, session_id);
    println!("⚠️ Em modo desenvolvimento - dados não salvos no banco para evitar restart");

    println!("💾 Chamando comando Tauri para salvar no localStorage...");
    let auth_data = format!("{{\"token\":\"{}\",\"sessionId\":\"{}\"}}", token, session_id);

    if let Err(e) = app_handle.emit("auth-data-received", auth_data) {
        eprintln!("❌ Erro ao emitir evento auth-data-received: {}", e);
    } else {
        println!("✅ Evento auth-data-received emitido com sucesso");
    }

    println!("🎨 Gerando página HTML...");

    // Página de sucesso limpa sem exibir dados sensíveis
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Auth Success</title>
            <meta http-equiv="refresh" content="3;url=http://localhost:3000/dashboard">
        </head>
        <body style="font-family: Arial, sans-serif; text-align: center; padding: 50px; background: #667eea; color: white; margin: 0;">
            <div style="background: rgba(255, 255, 255, 0.1); padding: 40px; border-radius: 20px; max-width: 500px; margin: 0 auto;">
                <h1 style="color: #4ade80; font-size: 32px; margin-bottom: 20px;">✅ Autenticação Realizada!</h1>

                <div style="color: #fbbf24; font-size: 18px; margin: 20px 0; padding: 15px; background: rgba(251, 191, 36, 0.1); border-radius: 10px;">
                    <p>✅ Autenticação realizada com sucesso. Redirecionando para o dashboard em 3 segundos...</p>
                    <p><small>Se não redirecionar automaticamente, clique no botão abaixo</small></p>
                </div>

                <div>
                    <button style="background: #3b82f6; color: white; border: none; padding: 12px 24px; border-radius: 8px; cursor: pointer; font-size: 16px; margin: 10px;" onclick="window.location.href='http://localhost:3000/dashboard'">Ir para Dashboard</button>
                    <button style="background: #4ade80; color: white; border: none; padding: 12px 24px; border-radius: 8px; cursor: pointer; font-size: 16px; margin: 10px;" onclick="window.close()">Fechar Janela</button>
                </div>
            </div>

            <script>
                console.log('Página de sucesso carregada');
                console.log('Redirecionando em 3 segundos...');

                setTimeout(() => {
                    console.log('Executando redirecionamento...');
                    window.location.href = 'http://localhost:3000/dashboard';
                }, 3000);
            </script>
        </body>
        </html>
        "#;

    println!("📤 Enviando resposta HTTP ({} bytes)", html.len());
    println!("🔍 Primeiros 200 caracteres da resposta: {}", &html[..html.len().min(200)]);

    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", html.len(), html);

    println!("📤 Enviando resposta para o cliente...");
    println!("✅ Resposta enviada com sucesso");
    println!("✅ Stream flushado com sucesso");

    response
}
