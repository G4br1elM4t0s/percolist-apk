use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::thread;
use tauri::{PhysicalSize, LogicalPosition, Manager};

use tauri::WebviewWindow;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, SetWindowLongW, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
};


#[tauri::command]
pub fn enable_click_through(window: WebviewWindow) {
    #[cfg(target_os = "windows")]
    unsafe {
        let hwnd = HWND(window.hwnd().unwrap().0 as isize);

        let ex_style: i32 = GetWindowLongW(hwnd, GWL_EXSTYLE);
        let new_style: i32 = ex_style
            | (WS_EX_LAYERED.0 as i32 | WS_EX_TRANSPARENT.0 as i32);

        SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);
    }
}

#[tauri::command]
pub fn disable_click_through(window: WebviewWindow) {
    #[cfg(target_os = "windows")]
    unsafe {
        let hwnd = HWND(window.hwnd().unwrap().0 as isize);

        let ex_style: i32 = GetWindowLongW(hwnd, GWL_EXSTYLE);
        let new_style: i32 =
            ex_style & !(WS_EX_TRANSPARENT.0 as i32); // remove apenas WS_EX_TRANSPARENT

        SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);
    }
}

pub static COLLAPSED_STATE: Mutex<bool> = Mutex::new(false);
static LAST_HOTKEY_TIME: Mutex<Option<Instant>> = Mutex::new(None);

/// Retorna o estado atual de colapso da janela
pub fn is_collapsed() -> bool {
    COLLAPSED_STATE.lock()
        .map(|state| *state)
        .unwrap_or(false)
}

/// Retorna a largura da tela principal
#[tauri::command]
pub fn get_screen_width(window: WebviewWindow) -> Result<u32, String> {
    // Obter o monitor atual da janela
    if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
        let size = monitor.size();
        println!("🔧 Largura da tela detectada: {}px", size.width);
        Ok(size.width)
    } else {
        println!("⚠️ Não foi possível obter o monitor, usando largura padrão: 1920px");
        Ok(1920)
    }
}

/// Função auxiliar para obter largura da tela sem parâmetros
pub fn get_screen_width_simple() -> u32 {
    // Para uso interno, retorna um valor padrão
    // A função principal get_screen_width deve ser usada via Tauri
    1920
}




#[tauri::command]
pub async fn toggle_collapse(window: tauri::WebviewWindow, is_collapsed: bool) -> Result<(), String> {
    println!("🔧 toggle_collapse chamado com is_collapsed: {}", is_collapsed);

    // Atualizar o estado global
    if let Ok(mut state) = COLLAPSED_STATE.lock() {
        *state = is_collapsed;
        println!("🔧 Estado global atualizado para: {}", *state);
    }

    let new_height = if is_collapsed { 85 } else { 85 };
    println!("🔧 Tentando redimensionar janela para altura: {}", new_height);

    // Obter largura da tela dinamicamente
    let screen_width = if let Some(monitor) = window.current_monitor().ok().flatten() {
        monitor.size().width
    } else {
        1920 // Fallback
    };

    // Primeiro, tentar redimensionar
    match window.set_size(PhysicalSize::new(screen_width, new_height)) {
        Ok(_) => println!("✓ Janela redimensionada com sucesso para {}px", new_height),
        Err(e) => {
            println!("✗ Erro ao redimensionar janela: {}", e);
            return Err(e.to_string());
        }
    }

    // Se colapsada, também mover para posição específica
    if is_collapsed {
        println!("🔧 Movendo janela colapsada para posição (-1, -1)");
        enable_click_through(window.clone());
        match window.set_position(LogicalPosition::new(-07.5,-53.0)) {
            Ok(_) => println!("✓ Janela movida para posição colapsada"),
            Err(e) => println!("✗ Erro ao mover janela: {}", e),
        }
    } else {
        println!("🔧 Restaurando janela para posição (0, 0)");
        disable_click_through(window.clone());
        match window.set_position(LogicalPosition::new(-07.5, -1.0)) {
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
    let state = is_collapsed();
    println!("🔧 get_collapsed_state retornando: {}", state);
    Ok(state)
}

/// Função síncrona para obter o estado de colapso
#[tauri::command]
pub fn get_collapsed_state_sync() -> bool {
    is_collapsed()
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

    // Obter largura da tela dinamicamente
    let screen_width = if let Some(monitor) = window.current_monitor().ok().flatten() {
        monitor.size().width
    } else {
        1920 // Fallback
    };

    match window.set_size(PhysicalSize::new(screen_width, 800)) {
        Ok(_) => {
            println!("✓ Janela expandida para 500px");

            // Garantir que a janela esteja visível e na posição correta
            match window.set_position(LogicalPosition::new(-07.5, -1.0)) {
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

    // Obter largura da tela dinamicamente
    let screen_width = if let Some(monitor) = window.current_monitor().ok().flatten() {
        monitor.size().width
    } else {
        1920 // Fallback
    };

    match window.set_size(PhysicalSize::new(screen_width, 85)) {
        Ok(_) => {
            println!("✓ Janela resetada para 85px");

            match window.set_position(LogicalPosition::new(-07.5, -1.0)) {
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
                                .args(["-i", "-r", window_id, "-e", "0,0,0,1920,75"])
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

    // Obter largura da tela dinamicamente
    let max_width = if let Some(monitor) = window.current_monitor().ok().flatten() {
        monitor.size().width as f64
    } else {
        1920.0 // Fallback
    };

    // Garantir que as dimensões não excedam o monitor e respeitem os limites mínimos/máximos
    let new_width = width.min(max_width).max(800.0);
    let new_height = height.min(monitor_size.height as f64).max(85.0);

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
