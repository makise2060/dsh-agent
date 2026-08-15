import { writable } from 'svelte/store';
import type { ProcessState } from '$lib/api/types';

export const processState = writable<ProcessState>({
  status: 'NotStarted',
  url: null,
  port: null,
  pid: null,
  error: null,
  started_at: null
});

export type NavRoute = 'main' | 'env' | 'plugins' | 'about';
export const currentRoute = writable<NavRoute>('main');
