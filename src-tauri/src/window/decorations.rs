#[cfg(windows)]
use windows::{
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
};
use std::time::Duration;
use std::thread;

#[cfg(windows)]
pub fn remove_window_decorations(window: tauri::WebviewWindow) {
    // Aguarda um momento para garantir que a janela foi criada
    thread::sleep(Duration::from_millis(100));

    // Obtém o HWND da janela - convertendo o ponteiro para isize
    let hwnd = HWND(window.hwnd().unwrap().0 as isize);

    unsafe {
        // Remove os estilos de borda
        let style = GetWindowLongW(hwnd, GWL_STYLE);
        let mask = (WS_CAPTION.0 | WS_THICKFRAME.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0 | WS_SYSMENU.0) as i32;
        let new_style = style & !mask;
        SetWindowLongW(hwnd, GWL_STYLE, new_style);

        // Adiciona estilos estendidos para transparência (apenas WS_EX_LAYERED, sem WS_EX_TRANSPARENT)
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        let ex_mask = WS_EX_LAYERED.0 as i32;
        let new_ex_style = ex_style | ex_mask;
        SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex_style);

        // Configura a transparência da janela
        let _ = SetLayeredWindowAttributes(
            hwnd,
            COLORREF(0), // RGB color key (0 para não usar color key)
            255, // Alpha (255 = totalmente opaco)
            LWA_ALPHA,
        );

        // Força o redraw e remove a moldura
        let _ = SetWindowPos(
            hwnd,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        );
    }
}
