// Shared types between Rust backend and Svelte frontend

export interface ProcessState {
  status: 'NotStarted' | 'Starting' | 'Running' | 'Stopping' | 'Stopped' | 'Failed';
  url: string | null;
  port: number | null;
  pid: number | null;
  error: string | null;
  started_at: string | null;
}

export interface NodeInfo {
  installed: boolean;
  version: string | null;
  path: string | null;
  meets_minimum: boolean | null;
}

export interface NpmInfo {
  installed: boolean;
  version: string | null;
  path: string | null;
}

export interface DshInfo {
  installed: boolean;
  version: string | null;
  path: string | null;
  latest_version: string | null;
  update_available: boolean;
}

export interface DshHomeInfo {
  exists: boolean;
  path: string;
  profiles_dir: boolean;
  sessions_dir: boolean;
}

export interface EnvState {
  node: NodeInfo;
  npm: NpmInfo;
  dsh: DshInfo;
  dsh_home: DshHomeInfo;
}

export interface UpdateInfo {
  current_version: string;
  latest_version: string;
  update_available: boolean;
  release_notes: string | null;
}

export interface PluginRepo {
  full_name: string;
  description: string | null;
  html_url: string;
  stargazers_count: number;
  topics: string[];
  updated_at: string;
  owner_avatar: string;
  license: string | null;
  installed: boolean;
}

export interface PluginSearchResult {
  repos: PluginRepo[];
  total_count: number;
  page: number;
  has_more: boolean;
}

export interface InstalledPlugin {
  name: string;
  version: string;
  path: string | null;
}
