<script lang="ts">
  import { currentRoute, processState, type NavRoute } from '$lib/stores/app';
  import { onMount } from 'svelte';
  import { onProcessStateChanged, getThemePreference, setThemePreference } from '$lib/api/tauri';
  import { theme, type ThemeMode } from '$lib/stores/theme';

  interface NavTab {
    id: NavRoute;
    label: string;
    /** lucide outline path */
    icon: string;
  }

  const tabs: NavTab[] = [
    {
      id: 'main',
      label: '主页',
      icon: '<path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/>'
    },
    {
      id: 'env',
      label: '环境',
      icon: '<rect x="2" y="2" width="20" height="8" rx="2" ry="2"/><rect x="2" y="14" width="20" height="8" rx="2" ry="2"/><line x1="6" y1="6" x2="6.01" y2="6"/><line x1="6" y1="18" x2="6.01" y2="18"/>'
    },
    {
      id: 'plugins',
      label: '插件',
      icon: '<path d="M19.439 7.85c-.049.322.059.648.289.878l1.568 1.568c.47.47.706 1.087.706 1.704s-.235 1.233-.706 1.704l-1.611 1.611a.98.98 0 0 1-.837.276c-.47-.07-.802-.48-.968-.925a2.501 2.501 0 1 0-3.214 3.214c.446.166.855.497.925.968a.979.979 0 0 1-.276.837l-1.61 1.61a2.404 2.404 0 0 1-3.417 0l-1.568-1.568a.98.98 0 0 0-.878-.289c-.322.05-.648-.059-.878-.29l-1.568-1.567a2.404 2.404 0 0 1 0-3.417l1.611-1.611a.98.98 0 0 1 .837-.276c.47.07.802.48.968.925a2.501 2.501 0 1 0 3.214-3.214c-.446-.166-.855-.497-.925-.968a.979.979 0 0 1 .276-.837l1.61-1.61a2.404 2.404 0 0 1 3.417 0l1.568 1.568c.23.23.556.338.878.289z"/>'
    },
    {
      id: 'about',
      label: '关于',
      icon: '<circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/>'
    }
  ];

  // 状态指示：图标 + 颜色底色 pill（比圆点明显，无需 tooltip）
  const statusStyles: Record<
    string,
    { dot: string; pill: string; label: string; icon: string }
  > = {
    NotStarted: {
      dot: 'bg-gray-400',
      pill: 'bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400 border-gray-200 dark:border-gray-600',
      label: '未启动',
      icon: '<circle cx="12" cy="12" r="10"/>'
    },
    Starting: {
      dot: 'bg-yellow-400 animate-pulse',
      pill: 'bg-yellow-50 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300 border-yellow-200 dark:border-yellow-800',
      label: '启动中',
      icon: '<path d="M21 12a9 9 0 1 1-6.2-8.56"/><path d="M21 3v6h-6"/>'
    },
    Running: {
      dot: 'bg-green-500',
      pill: 'bg-green-50 dark:bg-green-900/30 text-green-700 dark:text-green-300 border-green-200 dark:border-green-800',
      label: '运行中',
      icon: '<path d="M20 6 9 17l-5-5"/>'
    },
    Stopping: {
      dot: 'bg-yellow-400 animate-pulse',
      pill: 'bg-yellow-50 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300 border-yellow-200 dark:border-yellow-800',
      label: '停止中',
      icon: '<circle cx="12" cy="12" r="10"/><path d="M8 12h8"/>'
    },
    Stopped: {
      dot: 'bg-gray-400',
      pill: 'bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400 border-gray-200 dark:border-gray-600',
      label: '已停止',
      icon: '<circle cx="12" cy="12" r="10"/><path d="M8 12h8"/>'
    },
    Failed: {
      dot: 'bg-red-500',
      pill: 'bg-red-50 dark:bg-red-900/30 text-red-700 dark:text-red-300 border-red-200 dark:border-red-800',
      label: '错误',
      icon: '<circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>'
    }
  };

  let themeMode: ThemeMode = 'system';

  $: status = $processState.status;
  $: style = statusStyles[status] ?? statusStyles.NotStarted;

  // 主题三态控件：深色 / 浅色 / 跟随系统
  const themeOptions: { mode: ThemeMode; label: string; icon: string; title: string }[] = [
    { mode: 'light', label: '浅色', icon: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/>', title: '浅色主题' },
    { mode: 'dark', label: '深色', icon: '<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/>', title: '深色主题' },
    { mode: 'system', label: '跟随系统', icon: '<rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/>', title: '跟随系统' }
  ];

  async function handleThemeChange(mode: ThemeMode) {
    themeMode = mode;
    theme.applyMode(mode);
    try {
      await setThemePreference(mode);
    } catch (e) {
      console.error('写入主题偏好失败:', e);
    }
  }

  onMount(() => {
    // 读 dsh 外观偏好初始化外壳主题（默认 system）
    getThemePreference()
      .then((pref) => {
        const mode = (pref === 'light' || pref === 'dark' ? pref : 'system') as ThemeMode;
        themeMode = mode;
        theme.applyMode(mode);
      })
      .catch(() => {});

    const unlisten = onProcessStateChanged((state) => {
      processState.set(state);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  });
</script>

<nav class="flex h-10 items-center justify-between border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 px-4 shrink-0 transition-colors">
  <!-- Left: App name + tabs -->
  <div class="flex items-center gap-4">
    <span class="text-sm font-bold text-gray-800 dark:text-gray-200">DSH Agent</span>
    <div class="flex items-center gap-0.5">
      {#each tabs as tab}
        <!-- 图标 tab：未激活仅图标 + hover tooltip，激活图标 + 文字 -->
        <div class="relative group">
          <button
            class="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs font-medium transition-colors {$currentRoute === tab.id
              ? 'bg-brand-100 dark:bg-brand-900/50 text-brand-700 dark:text-brand-300'
              : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
            on:click={() => currentRoute.set(tab.id)}
          >
            <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              {@html tab.icon}
            </svg>
            {#if $currentRoute === tab.id}
              <span>{tab.label}</span>
            {/if}
          </button>
          <!-- hover tooltip：未激活时提示文字 -->
          {#if $currentRoute !== tab.id}
            <span class="pointer-events-none absolute left-1/2 top-full z-50 mt-1 -translate-x-1/2 whitespace-nowrap rounded bg-gray-800 px-1.5 py-0.5 text-[10px] text-white opacity-0 transition-opacity group-hover:opacity-100">
              {tab.label}
            </span>
          {/if}
        </div>
      {/each}
    </div>
  </div>

  <!-- Right: Status pill + theme -->
  <div class="flex items-center gap-2">
    <!-- 状态指示：图标 + 颜色底色 pill，直接显示文字 -->
    <span
      class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium {style.pill}"
    >
      <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        {@html style.icon}
      </svg>
      {style.label}
    </span>

    <!-- 分隔线 -->
    <span class="h-4 w-px bg-gray-200 dark:bg-gray-600"></span>

    <!-- 主题三态：深色 / 浅色 / 跟随系统，与 dsh 外观配置联动 -->
    <div class="flex items-center rounded-md border border-gray-200 dark:border-gray-600 overflow-hidden">
      {#each themeOptions as opt}
        <button
          class="flex items-center px-1.5 py-1 transition-colors {themeMode === opt.mode
            ? 'bg-brand-100 dark:bg-brand-900/50 text-brand-700 dark:text-brand-300'
            : 'text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
          on:click={() => handleThemeChange(opt.mode)}
          title={opt.title}
        >
          <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            {@html opt.icon}
          </svg>
        </button>
      {/each}
    </div>
  </div>
</nav>
