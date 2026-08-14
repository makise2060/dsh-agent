import { writable } from 'svelte/store';

export type Theme = 'light' | 'dark';

function getSystemTheme(): Theme {
  if (typeof window !== 'undefined' && window.matchMedia) {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  return 'light';
}

function createThemeStore() {
  const { subscribe, set } = writable<Theme>(getSystemTheme());

  // Apply theme to document element
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

  // Initialize
  apply(getSystemTheme());

  // Listen for system theme changes
  if (typeof window !== 'undefined' && window.matchMedia) {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.addEventListener('change', (e) => {
      const theme = e.matches ? 'dark' : 'light';
      apply(theme);
      set(theme);
    });
  }

  return {
    subscribe,
    init() {
      const theme = getSystemTheme();
      apply(theme);
      set(theme);
    }
  };
}

export const theme = createThemeStore();
