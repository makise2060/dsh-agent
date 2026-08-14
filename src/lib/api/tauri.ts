import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  ProcessState,
  EnvState,
  UpdateInfo,
  PluginSearchResult,
  InstalledPlugin
} from './types';

// ── Process Management ──────────────────────────────────────────

export async function startDsh(): Promise<ProcessState> {
  return invoke<ProcessState>('start_dsh');
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

// ── Event Listeners ─────────────────────────────────────────────

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
  cb: (p: { package: string; stage: string; message: string }) => void
): Promise<UnlistenFn> {
  return listen<{ package: string; stage: string; message: string }>(
    'plugin-install-progress',
    (e) => cb(e.payload)
  );
}
