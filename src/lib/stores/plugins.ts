import { writable } from 'svelte/store';
import type { PluginRepo, InstalledPlugin, BundleStatus } from '$lib/api/types';

export const marketRepos = writable<PluginRepo[]>([]);
export const installedPackages = writable<InstalledPlugin[]>([]);
export const pluginLoading = writable(false);
export const installingPackage = writable<string | null>(null);

// 界面插件全家桶（@linxin666/dsh-web-ui-all）
export const bundleStatus = writable<BundleStatus | null>(null);
export const bundleInstalling = writable(false);
export const bundleMessage = writable<string | null>(null);
