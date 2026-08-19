<script lang="ts">
  import { onMount } from 'svelte';
  import {
    searchPlugins,
    listInstalledPlugins,
    installPlugin,
    removePlugin,
    onPluginInstallProgress,
    checkBundleStatus,
    installBundle,
    verifyBundle,
    BUNDLE_PACKAGE
  } from '$lib/api/tauri';
  import {
    marketRepos,
    installedPackages,
    pluginLoading,
    installingPackage,
    bundleStatus,
    bundleInstalling,
    bundleMessage
  } from '$lib/stores/plugins';
  import { currentRoute } from '$lib/stores/app';
  import type { PluginRepo } from '$lib/api/types';

  let searchQuery = '';
  let sortBy: 'stars' | 'updated' | 'name' = 'stars';
  let page = 1;
  let hasMore = false;
  let totalCount = 0;
  let loaded = false;
  let retried = false;

  onMount(() => {
    loadInstalled();
    loadBundleStatus();
    let unlisten: (() => void) | undefined;

    onPluginInstallProgress((p) => {
      if (p.package === BUNDLE_PACKAGE) {
        // 全家桶事件：驱动卡片状态与进度
        if (p.stage === 'starting') {
          bundleInstalling.set(true);
          bundleMessage.set(p.message);
        } else if (p.stage === 'progress') {
          bundleMessage.set(p.message);
        } else if (p.stage === 'done' || p.stage === 'error') {
          bundleInstalling.set(false);
          bundleMessage.set(null);
          loadBundleStatus();
          loadInstalled();
        }
      } else if (p.stage === 'done') {
        installingPackage.set(null);
        loadInstalled();
        loadPlugins();
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  });

  // Load plugins when navigating to this tab (handles first visit and retry after failure)
  $: if ($currentRoute === 'plugins' && !loaded) {
    loaded = true;
    loadPlugins();
  }
  // Retry once if list is still empty after first load completes
  $: if ($currentRoute === 'plugins' && loaded && !retried && !$pluginLoading && $marketRepos.length === 0 && !searchQuery) {
    retried = true;
    loadPlugins();
  }

  async function loadPlugins() {
    pluginLoading.set(true);
    try {
      const result = await searchPlugins(searchQuery || undefined, sortBy, page);
      if (page === 1) {
        marketRepos.set(result.repos);
      } else {
        marketRepos.update((prev) => [...prev, ...result.repos]);
      }
      hasMore = result.has_more;
      totalCount = result.total_count;
    } catch (e) {
      console.error('Failed to load plugins:', e);
    } finally {
      pluginLoading.set(false);
    }
  }

  async function loadInstalled() {
    try {
      const installed = await listInstalledPlugins();
      installedPackages.set(installed);
    } catch (e) {
      console.error('Failed to list installed plugins:', e);
    }
  }

  async function loadBundleStatus() {
    try {
      bundleStatus.set(await checkBundleStatus());
    } catch (e) {
      console.error('Failed to check bundle status:', e);
    }
  }

  function isInstalled(repo: PluginRepo): boolean {
    return $installedPackages.some((p) => repo.full_name.includes(p.name));
  }

  async function handleInstall(repo: PluginRepo) {
    installingPackage.set(repo.full_name);
    try {
      await installPlugin(repo.full_name);
    } catch (e) {
      console.error('Install failed:', e);
      installingPackage.set(null);
    }
  }

  async function handleRemove(repo: PluginRepo) {
    const pkg = $installedPackages.find((p) => repo.full_name.includes(p.name));
    if (!pkg) return;
    try {
      await removePlugin(pkg.name);
      await loadInstalled();
    } catch (e) {
      console.error('Remove failed:', e);
    }
  }

  async function handleInstallBundle() {
    bundleInstalling.set(true);
    bundleMessage.set('正在安装界面插件全家桶…');
    try {
      bundleStatus.set(await installBundle());
      loadInstalled();
    } catch (e) {
      console.error('Bundle install failed:', e);
      bundleMessage.set(String(e));
    } finally {
      bundleInstalling.set(false);
    }
  }

  async function handleVerifyBundle() {
    try {
      bundleStatus.set(await verifyBundle());
    } catch (e) {
      console.error('Bundle verify failed:', e);
    }
  }

  function handleSearch() {
    page = 1;
    loadPlugins();
  }

  function handleSortChange(e: Event) {
    sortBy = (e.target as HTMLSelectElement).value as 'stars' | 'updated' | 'name';
    page = 1;
    loadPlugins();
  }

  function handleLoadMore() {
    page++;
    loadPlugins();
  }
</script>

<div class="flex h-full flex-col bg-gray-50 dark:bg-gray-900 transition-colors">
  <!-- Header -->
  <div class="flex items-center gap-3 border-b border-gray-200 dark:border-gray-700 px-4 py-2 transition-colors">
    <input
      type="text"
      placeholder="搜索插件..."
      bind:value={searchQuery}
      on:keydown={(e) => e.key === 'Enter' && handleSearch()}
      class="flex-1 rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-1.5 text-xs text-gray-700 dark:text-gray-300 focus:border-brand-500 focus:outline-none transition-colors"
    />
    <select
      bind:value={sortBy}
      on:change={handleSortChange}
      class="rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-2 py-1.5 text-xs text-gray-700 dark:text-gray-300 transition-colors"
    >
      <option value="stars">Stars</option>
      <option value="updated">最近更新</option>
      <option value="name">名称</option>
    </select>
    <button
      class="rounded-md bg-brand-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-brand-700"
      on:click={handleSearch}
    >
      搜索
    </button>
  </div>

  <!-- Plugin list -->
  <div class="flex-1 overflow-y-auto p-4">
    <!-- 界面插件全家桶卡片 -->
    <div class="mx-auto mb-4 max-w-3xl">
      <div class="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 transition-colors">
        <div class="flex items-start justify-between gap-4">
          <div class="flex-1">
            <div class="flex items-center gap-2">
              <h3 class="text-sm font-bold text-gray-800 dark:text-gray-200">界面插件全家桶</h3>
              <span
                class="rounded-full px-2 py-0.5 text-[10px] font-medium {$bundleStatus?.status === 'installed'
                  ? 'bg-green-100 dark:bg-green-900/50 text-green-700 dark:text-green-400'
                  : $bundleStatus?.status === 'needs_repair'
                    ? 'bg-orange-100 dark:bg-orange-900/50 text-orange-700 dark:text-orange-400'
                    : 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400'}"
              >
                {$bundleStatus?.status === 'installed'
                  ? '已安装'
                  : $bundleStatus?.status === 'needs_repair'
                    ? '需修复'
                    : '未安装'}
              </span>
            </div>
            <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
              一键安装鲸鱼娘、任务看板、皮肤中心、Git 图谱、右侧面板等整套界面插件
              {#if $bundleStatus?.installed_version || $bundleStatus?.expected_version}
                <span class="text-gray-400 dark:text-gray-500">
                  （{ $bundleStatus?.installed_version ? `当前 v${$bundleStatus.installed_version}` : `将安装 v${$bundleStatus.expected_version}` }）
                </span>
              {/if}
            </p>
            {#if $bundleStatus?.warning}
              <p class="mt-2 rounded-md border border-orange-200 dark:border-orange-800 bg-orange-50 dark:bg-orange-900/30 px-2 py-1.5 text-xs text-orange-700 dark:text-orange-400">
                {$bundleStatus.warning}
              </p>
            {/if}
            {#if $bundleInstalling}
              <div class="mt-3 space-y-1">
                <div class="h-1.5 w-full overflow-hidden rounded-full bg-gray-200 dark:bg-gray-700">
                  <div class="h-full bg-brand-600 transition-all duration-300"></div>
                </div>
                {#if $bundleMessage}
                  <p class="text-xs text-gray-500 dark:text-gray-400 font-mono break-all">{$bundleMessage}</p>
                {/if}
              </div>
            {/if}
          </div>
          <div class="shrink-0">
            {#if $bundleInstalling}
              <span class="text-xs text-gray-400 dark:text-gray-500">安装中...</span>
            {:else if $bundleStatus?.status === 'not_installed'}
              <button
                class="rounded-md bg-brand-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-brand-700"
                on:click={handleInstallBundle}
              >
                一键安装
              </button>
            {:else if $bundleStatus?.status === 'installed'}
              <div class="flex items-center gap-2">
                <button
                  class="rounded-md border border-gray-300 dark:border-gray-600 px-3 py-1.5 text-xs text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                  on:click={handleVerifyBundle}
                >
                  自检
                </button>
                <button
                  class="rounded-md border border-gray-300 dark:border-gray-600 px-3 py-1.5 text-xs text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                  on:click={handleInstallBundle}
                >
                  重装
                </button>
              </div>
            {:else}
              <button
                class="rounded-md bg-orange-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-orange-700"
                on:click={handleInstallBundle}
              >
                修复安装
              </button>
            {/if}
          </div>
        </div>
      </div>
    </div>

    {#if $pluginLoading && $marketRepos.length === 0}
      <div class="flex items-center justify-center py-12">
        <div class="h-6 w-6 animate-spin rounded-full border-2 border-brand-500 border-t-transparent"></div>
      </div>
    {:else if $marketRepos.length === 0}
      <div class="py-12 text-center text-sm text-gray-400 dark:text-gray-500">暂无插件</div>
    {:else}
      <div class="mx-auto max-w-3xl space-y-3">
        <p class="text-xs text-gray-400 dark:text-gray-500">共 {totalCount} 个插件</p>
        {#each $marketRepos as repo}
          <div class="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 transition-colors">
            <div class="flex items-start justify-between">
              <div class="flex-1">
                <a
                  href={repo.html_url}
                  target="_blank"
                  rel="noopener"
                  class="text-sm font-medium text-brand-600 dark:text-brand-400 hover:underline"
                >
                  {repo.full_name}
                </a>
                <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                  {repo.description || '无描述'}
                </p>
                <div class="mt-2 flex items-center gap-3 text-xs text-gray-400 dark:text-gray-500">
                  <span>⭐ {repo.stargazers_count}</span>
                  <span>{new Date(repo.updated_at).toLocaleDateString()}</span>
                  {#if repo.license}<span>{repo.license}</span>{/if}
                </div>
              </div>
              <div>
                {#if $installingPackage === repo.full_name}
                  <span class="text-xs text-gray-400 dark:text-gray-500">安装中...</span>
                {:else if isInstalled(repo)}
                  <button
                    class="rounded-md border border-red-300 dark:border-red-700 px-3 py-1 text-xs text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors"
                    on:click={() => handleRemove(repo)}
                  >
                    移除
                  </button>
                {:else}
                  <button
                    class="rounded-md bg-brand-600 px-3 py-1 text-xs font-medium text-white hover:bg-brand-700"
                    on:click={() => handleInstall(repo)}
                  >
                    安装
                  </button>
                {/if}
              </div>
            </div>
          </div>
        {/each}
        {#if hasMore}
          <button
            class="w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 py-2 text-xs text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
            on:click={handleLoadMore}
          >
            加载更多
          </button>
        {/if}
      </div>
    {/if}
  </div>
</div>
