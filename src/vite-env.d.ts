/// <reference types="vite/client" />

/**
 * 互联 · Vite 环境变量类型
 * `VITE_MOCK=1` 时 api.ts 走 mock 实现；TAURI_ENV_PLATFORM 由 Tauri CLI 注入。
 */
interface ImportMetaEnv {
  readonly VITE_MOCK?: string;
  readonly TAURI_ENV_PLATFORM?: string;
}
