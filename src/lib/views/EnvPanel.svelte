<script lang="ts">
  import { onMount } from 'svelte';
  import { envState, envChecking } from '$lib/stores/env';
  import { checkEnvironment, installDsh, checkDshUpdate, onInstallProgress } from '$lib/api/tauri';
  import type { UpdateInfo } from '$lib/api/types';
  import EnvCard from '$lib/components/EnvCard.svelte';
  import DshUpdateCard from '$lib/components/DshUpdateCard.svelte';
  import Button from '$lib/components/Button.svelte';

  let dshUpdate: UpdateInfo | null = null;
  let checkingDsh = false;
  let installing = false;
  let installLog: string[] = [];

  onMount(() => {
    refresh();
    let unlisten: (() => void) | undefined;

    onInstallProgress((p) => {
      installLog = [...installLog, p.message];
      if (p.stage === 'done') {
        installing = false;
        refresh();
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  });

  async function refresh() {
    envChecking.set(true);
    try {
      const state = await checkEnvironment();
      envState.set(state);
      // Also refresh dsh update info if it was checked before
      if (dshUpdate) {
        await handleCheckDsh();
      }
    } catch (e) {
      console.error('Env check failed:', e);
    } finally {
      envChecking.set(false);
    }
  }

  async function handleCheckDsh() {
    checkingDsh = true;
    try {
      dshUpdate = await checkDshUpdate();
    } catch (e) {
      console.error(e);
    } finally {
      checkingDsh = false;
    }
  }

  async function handleInstallDsh() {
    installing = true;
    installLog = [];
    try {
      await installDsh();
      await refresh();
      await handleCheckDsh();
    } catch (e) {
      installLog = [...installLog, `Error: ${e}`];
      installing = false;
    }
  }

  $: env = $envState;

  // 图标常量（lucide outline）
  const ICON_NODE = '<circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>';
  const ICON_NPM = '<path d="M16.5 9.4 7.55 4.24"/><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><path d="M3.29 7 12 12l8.71-5"/><path d="M12 22V12"/>';
  const ICON_DSH = '<path d="M12 2v4"/><path d="M12 18v4"/><path d="M4.93 4.93l2.83 2.83"/><path d="M16.24 16.24l2.83 2.83"/><path d="M2 12h4"/><path d="M18 12h4"/><path d="M4.93 19.07l2.83-2.83"/><path d="M16.24 7.76l2.83-2.83"/>';
  const ICON_FOLDER = '<path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/>';
  const ICON_REFRESH = '<polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>';

  // 高亮规则：node 未装=error、版本不足=warn；npm 未装=error；dsh 未装=error、有更新=warn；DSH_HOME 未创建=warn
</script>

<div class="h-full overflow-y-auto bg-gray-50 dark:bg-gray-900 p-6 transition-colors">
  <div class="mx-auto max-w-2xl space-y-5">
    <div class="flex items-center justify-between">
      <h1 class="text-lg font-bold text-gray-800 dark:text-gray-200">运行环境</h1>
      <Button
        variant="secondary"
        size="sm"
        icon={ICON_REFRESH}
        loading={$envChecking}
        on:click={refresh}
      >
        {$envChecking ? '检查中...' : '重新检查'}
      </Button>
    </div>

    <!-- Node.js -->
    <EnvCard
      title="Node.js"
      icon={ICON_NODE}
      badgeStatus={!env.node.installed ? 'error' : env.node.meets_minimum ? 'ok' : 'warn'}
      badgeLabel={!env.node.installed ? '未安装' : env.node.meets_minimum ? `v${env.node.version}` : `v${env.node.version} (需 ≥22)`}
      highlight={!env.node.installed ? 'error' : env.node.meets_minimum ? 'none' : 'warn'}
      path={env.node.path}
    >
      {#if !env.node.installed || !env.node.meets_minimum}
        <p class="text-xs text-gray-500 dark:text-gray-400">
          {!env.node.installed ? '未检测到 Node.js，引导流程会自动下载便携版。' : `当前版本 v${env.node.version} 不满足要求（需 ≥22.19 或 ≥24）。`}
        </p>
      {/if}
      <div slot="actions">
        {#if !env.node.installed || !env.node.meets_minimum}
          <a
            href="https://nodejs.org/"
            target="_blank"
            rel="noopener"
            class="inline-flex items-center gap-1.5 rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          >
            下载 Node.js →
          </a>
        {/if}
      </div>
    </EnvCard>

    <!-- npm -->
    <EnvCard
      title="npm"
      icon={ICON_NPM}
      badgeStatus={env.npm.installed ? 'ok' : 'error'}
      badgeLabel={env.npm.installed ? `v${env.npm.version}` : '未安装'}
      highlight={env.npm.installed ? 'none' : 'error'}
      path={env.npm.path}
    >
      {#if !env.npm.installed}
        <p class="text-xs text-gray-500 dark:text-gray-400">npm 随 Node.js 一起安装，引导流程会处理。</p>
      {/if}
    </EnvCard>

    <!-- dsh 基础信息 -->
    <EnvCard
      title="dsh (DeepSeek Harness)"
      icon={ICON_DSH}
      badgeStatus={!env.dsh.installed ? 'error' : env.dsh.update_available ? 'warn' : 'ok'}
      badgeLabel={!env.dsh.installed ? '未安装' : env.dsh.update_available ? `v${env.dsh.version} (有更新)` : `v${env.dsh.version}`}
      highlight={!env.dsh.installed ? 'error' : env.dsh.update_available ? 'warn' : 'none'}
      path={env.dsh.path}
    >
      {#if !env.dsh.installed}
        <p class="text-xs text-gray-500 dark:text-gray-400">dsh 未安装，引导流程会自动安装；也可在下方更新卡片手动安装。</p>
      {/if}
    </EnvCard>

    <!-- dsh 更新/安装 -->
    <DshUpdateCard
      update={dshUpdate}
      dsh={env.dsh}
      checking={checkingDsh}
      installing={installing}
      installLog={installLog}
      onCheck={handleCheckDsh}
      onInstall={handleInstallDsh}
    />

    <!-- DSH_HOME -->
    <EnvCard
      title="DSH_HOME"
      icon={ICON_FOLDER}
      badgeStatus={env.dsh_home.exists ? 'ok' : 'warn'}
      badgeLabel={env.dsh_home.exists ? '就绪' : '未创建'}
      highlight={env.dsh_home.exists ? 'none' : 'warn'}
      path={env.dsh_home.path}
    >
      {#if env.dsh_home.exists}
        <p class="text-xs text-gray-500 dark:text-gray-400">
          profiles: {env.dsh_home.profiles_dir ? '✅' : '❌'} ·
          sessions: {env.dsh_home.sessions_dir ? '✅' : '❌'}
        </p>
      {:else}
        <p class="text-xs text-gray-500 dark:text-gray-400">首次运行 dsh 时自动创建。</p>
      {/if}
    </EnvCard>
  </div>
</div>
