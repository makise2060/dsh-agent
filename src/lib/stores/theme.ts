import { writable } from 'svelte/store';

export type Theme = 'light' | 'dark';
export type ThemeMode = 'light' | 'dark' | 'system';

function getSystemTheme(): Theme {
  if (typeof window !== 'undefined' && window.matchMedia) {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  return 'light';
}

function createThemeStore() {
  // 当前生效主题（只可能是 light/dark）
  const { subscribe, set } = writable<Theme>(getSystemTheme());

  function apply(theme: Theme) {
    if (typeof document !== 'undefined') {
      const root = document.documentElement;
      if (theme === 'dark') {
        root.classList.add('dark');
        root.setAttribute('data-theme', 'dark');
      } else {
        root.classList.remove('dark');
        root.setAttribute('data-theme', 'light');
      }
    }
  }

  function resolve(mode: ThemeMode): Theme {
    return mode === 'system' ? getSystemTheme() : mode;
  }

  // Initialize
  apply(getSystemTheme());

  // 当前模式（light/dark/system），默认 system
  let mode: ThemeMode = 'system';

  // 系统主题变化：仅 system 模式跟随
  if (typeof window !== 'undefined' && window.matchMedia) {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.addEventListener('change', (e) => {
      if (mode === 'system') {
        const theme = e.matches ? 'dark' : 'light';
        apply(theme);
        set(theme);
      }
    });
  }

  return {
    subscribe,
    /** 应用某个模式（不持久化，由调用方负责写入 dsh settings） */
    applyMode(m: ThemeMode) {
      mode = m;
      const theme = resolve(m);
      apply(theme);
      set(theme);
    },
    getMode(): ThemeMode {
      return mode;
    },
    init() {
      const theme = getSystemTheme();
      apply(theme);
      set(theme);
    }
  };
}

export const theme = createThemeStore();
