declare module '@tauri-apps/plugin-updater' {
  export interface Update {
    available: boolean;
    version: string;
    body: string;
    downloadAndInstall(): Promise<void>;
  }

  export function check(): Promise<Update | null>;
}
