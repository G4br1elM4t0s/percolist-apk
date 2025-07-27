#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::process::Command;
use std::thread;
use std::time::Duration;
use std::sync::Arc;
use tauri::{Manager, PhysicalSize, PhysicalPosition, Emitter};
use global_hotkey::{GlobalHotKeyManager, hotkey::HotKey, GlobalHotKeyEvent};

// Módulos organizados
mod models;
mod database;
mod hotkey;
mod auth;
mod auth_server;
mod tasks;
mod pomodoro;
mod window;
mod system;

// Imports dos módulos
use database::{init_database, DatabaseState};
use auth::*;
use tasks::*;
use pomodoro::*;
use window::*;
use system::*;


#[cfg(windows)]
use window::remove_window_decorations;

fn main() {
    println!("Iniciando aplicação Percolist...");

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
        let hotkey = HotKey::new(Some(global_hotkey::hotkey::Modifiers::ALT), global_hotkey::hotkey::Code::KeyC);
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

    let db_connection = init_database().expect("Database initialization failed");
    let db_state = DatabaseState {
        connection: Arc::new(std::sync::Mutex::new(db_connection)),
    };

    // Criar estado do servidor auth
    let auth_server_state = auth::AuthServerState::new();

    tauri::Builder::default()
        .manage(db_state)
        .manage(auth_server_state)
                                        .setup(|_app| {
            println!("🚀 App iniciado com sucesso!");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth commands
            get_saved_auth_data,
            clear_saved_auth_data,
            test_auth_callback,
            verify_auth_storage,
            check_auth_data_received_command,
            start_auth_server,
            stop_auth_server,
            auth_server_status,
            test_auth_server,

            // Tasks CRUD
            load_tasks,
            add_task,
            update_task,
            delete_task,
            get_task_by_id,
            increment_task_count,
            get_today_tasks,

            // Tasks com sessões e Pomodoro
            start_task,
            complete_task,
            pause_task,
            resume_task,
            check_pomodoro_sessions,
            load_tasks_with_sessions_command,

            // Time tracking
            get_task_remaining_time,
            get_task_total_worked_time,

            // Scheduling
            reschedule_tasks,

            // Window management
            toggle_collapse,
            get_collapsed_state,
            test_hotkey_manually,
            expand_window_for_modal,
            reset_window_size,
            adjust_window_size,

            // System controls
            get_system_volume,
            set_system_volume,
            get_system_mute_status,
            toggle_system_mute,
        ])
        .setup(move |app| {
            let handle = app.handle();
            let window = handle.get_webview_window("main").unwrap();

            // Iniciar servidor auth com melhor tratamento de erro
            println!("🔧 Iniciando servidor de autenticação...");
            let auth_server = auth_server::AuthServer::new();
            let db_connection_for_server = handle.state::<DatabaseState>().connection.clone();

            // Tentar iniciar o servidor
            auth_server.start(db_connection_for_server, handle.clone());

            // Aguardar um pouco e verificar se iniciou
            std::thread::sleep(std::time::Duration::from_millis(500));

            // Verificar se o servidor está rodando
            match std::net::TcpStream::connect_timeout(
                &"127.0.0.1:1420".parse().unwrap(),
                std::time::Duration::from_millis(1000)
            ) {
                Ok(_) => println!("✅ Servidor de autenticação confirmado na porta 1420"),
                Err(_) => {
                    eprintln!("⚠️ Servidor de autenticação pode não ter iniciado corretamente");
                    eprintln!("💡 Tente usar o comando 'start_auth_server' manualmente");
                }
            }

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
                                match tasks::scheduling::reschedule_incomplete_tasks(&conn) {
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
                                if !window::management::should_process_hotkey() {
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
                        if line.contains("Percolist") || line.contains("app") {
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
