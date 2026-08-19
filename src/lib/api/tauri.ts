import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  ProcessState,
  EnvState,
  UpdateInfo,
  PluginSearchResult,
  InstalledPlugin,
  BundleStatus
} from './types';

// ── Process Management ──────────────────────────────────────────

export async function startDsh(): Promise<ProcessState> {
  return invoke<ProcessState>('start_dsh');
}

/** 引导编排：检测/下载 Node → 装 dsh → 装插件 → 启动 dsh，进度走 bootstrap:progress 事件 */
export async function startBootstrap(): Promise<void> {
  return invoke('start_bootstrap');
}

export async function stopDsh(): Promise<void> {
  return invoke('stop_dsh');
}

export async function getDshStatus(): Promise<ProcessState> {
  return invoke<ProcessState>('get_dsh_status');
}

export async function restartDsh(): Promise<ProcessState> {
  return invoke<ProcessState>('restart_dsh');
}

// ── Environment Check ───────────────────────────────────────────

export async function checkEnvironment(): Promise<EnvState> {
  return invoke<EnvState>('check_environment');
}

export async function checkNodeVersion() {
  return invoke('check_node_version');
}

export async function checkDshVersion() {
  return invoke('check_dsh_version');
}

// ── Install / Update ────────────────────────────────────────────

export async function installDsh(): Promise<void> {
  return invoke('install_dsh');
}

export async function checkDshUpdate(): Promise<UpdateInfo> {
  return invoke<UpdateInfo>('check_dsh_update');
}

export async function checkAppUpdate(): Promise<UpdateInfo> {
  return invoke<UpdateInfo>('check_app_update');
}

export async function getLogsDir(): Promise<string> {
  return invoke<string>('get_logs_dir');
}

// ── Close Behavior ──────────────────────────────────────────────

/** 关闭确认框三选：quit / tray / cancel。remember=true 时记住（cancel 永不记住） */
export async function resolveClose(action: string, remember: boolean): Promise<void> {
  return invoke('resolve_close', { action, remember });
}

export async function getCloseAction(): Promise<string | null> {
  return invoke<string | null>('get_close_action');
}

// ── Task-completion notification ───────────────────────────────

export async function getNotifyOnDone(): Promise<boolean> {
  return invoke<boolean>('get_notify_on_done');
}

export async function setNotifyOnDone(enabled: boolean): Promise<void> {
  return invoke('set_notify_on_done', { enabled });
}

// ── Theme sync ─────────────────────────────────────────────────

/** 读 dsh 外观偏好：light / dark / system */
export async function getThemePreference(): Promise<string> {
  return invoke<string>('get_theme_preference');
}

/** 写 dsh 外观偏好（light / dark / system），写回 settings.yaml */
export async function setThemePreference(preference: string): Promise<string> {
  return invoke<string>('set_theme_preference', { preference });
}

// ── Plugin Market ────────────────────────────────────────────────

export async function searchPlugins(
  query?: string,
  sort?: 'stars' | 'updated' | 'name',
  page?: number
): Promise<PluginSearchResult> {
  return invoke<PluginSearchResult>('search_plugins', {
    query: query ?? null,
    sort: sort ?? null,
    page: page ?? null
  });
}

export async function listInstalledPlugins(): Promise<InstalledPlugin[]> {
  return invoke<InstalledPlugin[]>('list_installed_plugins');
}

export async function installPlugin(packageName: string): Promise<void> {
  return invoke('install_plugin', { packageName });
}

export async function removePlugin(packageName: string): Promise<void> {
  return invoke('remove_plugin', { packageName });
}

export async function activatePlugin(pluginId: string, pluginName: string): Promise<void> {
  return invoke('activate_plugin', { pluginId, pluginName });
}

// ── Plugin Bundle (dsh-web-ui all-in-one) ───────────────────────

/** 界面插件全家桶聚合包名 */
export const BUNDLE_PACKAGE = '@linxin666/dsh-web-ui-all';

export async function checkBundleStatus(): Promise<BundleStatus> {
  return invoke<BundleStatus>('check_bundle_status');
}

export async function installBundle(): Promise<BundleStatus> {
  return invoke<BundleStatus>('install_bundle');
}

export async function verifyBundle(): Promise<BundleStatus> {
  return invoke<BundleStatus>('verify_bundle');
}

// ── Event Listeners ─────────────────────────────────────────────

/** 引导进度事件（stage/label/detail/fraction/index/total/transient） */
export interface BootstrapProgress {
  stage: string;
  label: string;
  detail: string | null;
  fraction: number | null;
  index: number;
  total: number;
  transient: boolean;
}

export function onBootstrapProgress(cb: (p: BootstrapProgress) => void): Promise<UnlistenFn> {
  return listen<BootstrapProgress>('bootstrap:progress', (e) => cb(e.payload));
}

export function onBootstrapReady(cb: (payload: { url: string }) => void): Promise<UnlistenFn> {
  return listen<{ url: string }>('bootstrap:ready', (e) => cb(e.payload));
}

export function onBootstrapFailed(cb: (message: string) => void): Promise<UnlistenFn> {
  return listen<string>('bootstrap:failed', (e) => cb(e.payload));
}

export function onBootstrapWarning(cb: (payload: { message: string }) => void): Promise<UnlistenFn> {
  return listen<{ message: string }>('bootstrap:warning', (e) => cb(e.payload));
}

export function onProcessStateChanged(cb: (state: ProcessState) => void): Promise<UnlistenFn> {
  return listen<ProcessState>('process-state-changed', (e) => cb(e.payload));
}

export function onDshStdout(cb: (line: string) => void): Promise<UnlistenFn> {
  return listen<string>('dsh-stdout', (e) => cb(e.payload));
}

export function onInstallProgress(
  cb: (p: { stage: string; message: string; percent?: number }) => void
): Promise<UnlistenFn> {
  return listen<{ stage: string; message: string; percent?: number }>('install-progress', (e) =>
    cb(e.payload)
  );
}

export function onPluginInstallProgress(
  cb: (p: { package: string; stage: string; message: string; percent?: number }) => void
): Promise<UnlistenFn> {
  return listen<{ package: string; stage: string; message: string; percent?: number }>(
    'plugin-install-progress',
    (e) => cb(e.payload)
  );
}
