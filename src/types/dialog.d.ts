declare module '@tauri-apps/plugin-dialog' {
  export interface DialogOptions {
    title?: string;
    kind?: 'info' | 'warning' | 'error';
    okLabel?: string;
    cancelLabel?: string;
  }

  export function ask(message: string, options?: DialogOptions): Promise<boolean>;
  export function message(message: string, options?: DialogOptions): Promise<void>;
}
