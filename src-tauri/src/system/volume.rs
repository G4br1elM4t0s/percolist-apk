

#[tauri::command]
pub async fn get_system_volume() -> Result<i32, String> {
    #[cfg(target_os = "linux")]
    {
        // Usar pactl para obter o volume no Linux
        match Command::new("pactl")
            .args(["get-sink-volume", "@DEFAULT_SINK@"])
            .output()
        {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Procurar por porcentagem no formato "XX%"
                for line in output_str.lines() {
                    if let Some(percent_pos) = line.find('%') {
                        // Procurar o número antes do %
                        let before_percent = &line[..percent_pos];
                        if let Some(space_pos) = before_percent.rfind(' ') {
                            let volume_str = &before_percent[space_pos + 1..];
                            if let Ok(volume) = volume_str.parse::<i32>() {
                                return Ok(volume);
                            }
                        }
                    }
                }
                Ok(50) // Fallback
            }
            Err(_) => Ok(50) // Fallback se pactl não estiver disponível
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Para outros sistemas, retornar um valor padrão por enquanto
        Ok(50)
    }
}

#[tauri::command]
pub async fn set_system_volume(volume: i32) -> Result<(), String> {
    let clamped_volume = volume.clamp(0, 100);

    #[cfg(target_os = "linux")]
    {
        // Usar pactl para definir o volume no Linux
        match Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", clamped_volume)])
            .output()
        {
            Ok(_) => {
                println!("🔊 Volume definido para {}%", clamped_volume);
                Ok(())
            }
            Err(e) => {
                println!("✗ Erro ao definir volume: {}", e);
                Err(format!("Erro ao definir volume: {}", e))
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("🔊 Volume seria definido para {}% (não implementado para este OS)", clamped_volume);
        Ok(())
    }
}

#[tauri::command]
pub async fn get_system_mute_status() -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    {
        match Command::new("pactl")
            .args(["get-sink-mute", "@DEFAULT_SINK@"])
            .output()
        {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                Ok(output_str.trim() == "yes")
            }
            Err(_) => Ok(false)
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(false)
    }
}

#[tauri::command]
pub async fn toggle_system_mute() -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    {
        match Command::new("pactl")
            .args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
            .output()
        {
            Ok(_) => {
                // Obter o novo status após o toggle
                get_system_mute_status().await
            }
            Err(e) => Err(format!("Erro ao alternar mute: {}", e))
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(false)
    }
}
