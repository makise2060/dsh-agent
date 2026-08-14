import { writable } from 'svelte/store';
import type { PluginRepo, InstalledPlugin } from '$lib/api/types';

export const marketRepos = writable<PluginRepo[]>([]);
export const installedPackages = writable<InstalledPlugin[]>([]);
export const pluginLoading = writable(false);
export const installingPackage = writable<string | null>(null);
