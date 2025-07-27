use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}, GlobalHotKeyEvent};

pub fn should_process_hotkey(_event: &GlobalHotKeyEvent) -> bool {
    // Implementação básica - sempre retorna true
    // Pode ser expandida para verificar condições específicas
    true
}

pub fn setup_global_hotkeys() -> Result<GlobalHotKeyManager, Box<dyn std::error::Error>> {
    let manager = GlobalHotKeyManager::new()?;

    // Configurar hotkey para toggle da janela (Alt+C)
    let hotkey = HotKey::new(Some(Modifiers::ALT), Code::KeyC);
    manager.register(hotkey)?;

    Ok(manager)
}
