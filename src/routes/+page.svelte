<script>
  import { onMount } from 'svelte';
  import { currentScreen } from '$lib/stores/settings.js';
  import { checkForUpdates } from '$lib/tauri.js';
  import Welcome from '$lib/components/Welcome.svelte';
  import Processing from '$lib/components/Processing.svelte';
  import Done from '$lib/components/Done.svelte';

  let update = $state(null);
  let installing = $state(false);
  let restartReady = $state(false);

  onMount(async () => {
    update = await checkForUpdates();
  });

  async function install() {
    if (!update) return;
    installing = true;
    try {
      await update.downloadAndInstall();
      restartReady = true;
    } catch (e) {
      console.error('Update install failed:', e);
    }
    installing = false;
  }
</script>

{#if update && !restartReady}
  <div class="update-bar">
    <span>Fix My Takeout {update.version} is available.</span>
    <button onclick={install} disabled={installing}>
      {installing ? 'Installing...' : 'Update now'}
    </button>
  </div>
{/if}
{#if restartReady}
  <div class="update-bar">
    <span>Update installed. Restart to apply.</span>
  </div>
{/if}

{#if $currentScreen === 'welcome'}
  <Welcome />
{:else if $currentScreen === 'processing'}
  <Processing />
{:else if $currentScreen === 'done'}
  <Done />
{/if}

<style>
  :global(:root) {
    --bg: #ffffff;
    --surface: #ffffff;
    --border: #F6EAE9;
    --text: #2c2c2c;
    --secondary: #a39e9e;
    --accent: #FE655F;
    --accent-light: #FEE8E7;
    --accent-hover: #e8524c;
    --accent-faint: #FFF5F4;
  }
  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --bg: #1a1717;
      --surface: #252121;
      --border: #3a3232;
      --text: #f5f0f0;
      --secondary: #8a8080;
      --accent: #FE655F;
      --accent-light: #3a2525;
      --accent-hover: #ff7a75;
      --accent-faint: #2a1e1e;
    }
  }
  :global(*) { margin: 0; padding: 0; box-sizing: border-box; }
  :global(body) {
    font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-size: 14px;
    line-height: 1.5;
    color: var(--text);
    background: transparent;
    -webkit-font-smoothing: antialiased;
  }
  .update-bar {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 6px 16px;
    background: var(--accent);
    color: #fff;
    font-size: 13px;
  }
  .update-bar button {
    background: rgba(255,255,255,0.2);
    color: #fff;
    border: 1px solid rgba(255,255,255,0.3);
    border-radius: 4px;
    padding: 2px 10px;
    font-size: 12px;
    cursor: pointer;
  }
  .update-bar button:hover { background: rgba(255,255,255,0.3); }
  .update-bar button:disabled { opacity: 0.6; cursor: default; }
</style>
