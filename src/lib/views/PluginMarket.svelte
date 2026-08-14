<script lang="ts">
  import { onMount } from 'svelte';
  import {
    searchPlugins,
    listInstalledPlugins,
    installPlugin,
    removePlugin,
    onPluginInstallProgress
  } from '$lib/api/tauri';
  import { marketRepos, installedPackages, pluginLoading, installingPackage } from '$lib/stores/plugins';
  import type { PluginRepo } from '$lib/api/types';

  let searchQuery = '';
  let sortBy: 'stars' | 'updated' | 'name' = 'stars';
  let page = 1;
  let hasMore = false;
  let totalCount = 0;

  onMount(async () => {
    await loadPlugins();
    await loadInstalled();
    const unlisten = await onPluginInstallProgress((p) => {
      if (p.stage === 'done') {
        installingPackage.set(null);
        loadInstalled();
        loadPlugins();
      }
    });
    return () => unlisten();
  });

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
