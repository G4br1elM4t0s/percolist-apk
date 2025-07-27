use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::thread;
use tauri::{PhysicalSize, LogicalPosition};

pub static COLLAPSED_STATE: Mutex<bool> = Mutex::new(false);
static LAST_HOTKEY_TIME: Mutex<Option<Instant>> = Mutex::new(None);

#[tauri::command]
pub async fn toggle_collapse(window: tauri::WebviewWindow, is_collapsed: bool) -> Result<(), String> {
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
pub async fn get_collapsed_state() -> Result<bool, String> {
    let state = COLLAPSED_STATE.lock()
        .map(|state| *state)
        .map_err(|e| e.to_string())?;
    println!("🔧 get_collapsed_state retornando: {}", state);
    Ok(state)
}

#[tauri::command]
pub async fn test_hotkey_manually(window: tauri::WebviewWindow) -> Result<(), String> {
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

pub fn should_process_hotkey() -> bool {
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
pub async fn expand_window_for_modal(window: tauri::WebviewWindow) -> Result<(), String> {
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
                        if line.contains("Percolist") || line.contains("app") || line.contains("percolist") {
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
pub async fn reset_window_size(window: tauri::WebviewWindow) -> Result<(), String> {
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
                        if line.contains("Percolist") || line.contains("app") || line.contains("percolist") {
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

#[tauri::command]
pub async fn adjust_window_size(window: tauri::WebviewWindow, width: f64, height: f64) -> Result<(), String> {
    let current_position = window.outer_position().map_err(|e| e.to_string())?;
    let monitor = window.current_monitor().map_err(|e| e.to_string())?.unwrap();
    let monitor_size = monitor.size();

    // Garantir que as dimensões não excedam o monitor e respeitem os limites mínimos/máximos
    let new_width = width.min(1920.0).max(800.0);
    let new_height = height.min(monitor_size.height as f64).max(70.0);

    window
        .set_size(tauri::PhysicalSize::new(new_width, new_height))
        .map_err(|e| e.to_string())?;

    // Manter a posição Y e centralizar horizontalmente
    let x = (monitor_size.width as f64 - new_width) / 2.0;
    window
        .set_position(tauri::PhysicalPosition::new(
            x as i32,
            current_position.y
        ))
        .map_err(|e| e.to_string())?;

    Ok(())
}
