import { writable } from 'svelte/store';
import type { EnvState } from '$lib/api/types';

export const envState = writable<EnvState>({
  node: { installed: false, version: null, path: null, meets_minimum: null },
  npm: { installed: false, version: null, path: null },
  dsh: {
    installed: false,
    version: null,
    path: null,
    latest_version: null,
    update_available: false
  },
  dsh_home: { exists: false, path: '', profiles_dir: false, sessions_dir: false }
});

export const envChecking = writable(false);
