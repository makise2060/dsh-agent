<script lang="ts">
  export let message = '正在启动 DeepSeek Harness...';
</script>

<div class="flex h-full flex-col items-center justify-center gap-4 bg-gray-50">
  <div class="flex items-center gap-3">
    <div class="h-6 w-6 animate-spin rounded-full border-2 border-brand-500 border-t-transparent"></div>
    <span class="text-sm text-gray-600">{message}</span>
  </div>
</div>
