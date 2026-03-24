<script>
  import { sourcePath, outputPath, currentScreen } from '../stores/settings.js';
  import { resetProgress } from '../stores/progress.js';
  import { selectFolder, startProcessing, getDiskInfo, checkExistingSession, setupListeners } from '../tauri.js';

  let diskInfo = $state(null);
  let sessionInfo = $state(null);
  let error = $state('');
  let starting = $state(false);

  async function pickSource() {
    const path = await selectFolder('Select your export folder');
    if (path) $sourcePath = path;
  }

  async function pickOutput() {
    const path = await selectFolder('Choose where to save the organized library');
    if (path) {
      $outputPath = path;
      try {
        diskInfo = await getDiskInfo(path);
        sessionInfo = await checkExistingSession(path);
      } catch (_) {}
    }
  }

  async function handleStart() {
    if (!$sourcePath || !$outputPath) return;
    error = '';
    starting = true;
    resetProgress();
    try {
      await setupListeners();
      await startProcessing($sourcePath, $outputPath);
      $currentScreen = 'processing';
    } catch (e) {
      error = String(e);
      starting = false;
    }
  }

  function fmt(bytes) {
    if (!bytes) return '0 GB';
    const gb = bytes / 1073741824;
    if (gb >= 1) return `${gb.toFixed(1)} GB`;
    return `${(bytes / 1048576).toFixed(0)} MB`;
  }

  // Estimate: source zips ≈ compressed, need ~1.5x space
  let spaceOk = $derived(!diskInfo || diskInfo.available_bytes > 5_000_000_000);
  let ready = $derived(!!$sourcePath && !!$outputPath);
</script>

<div class="welcome">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="drag-region" data-tauri-drag-region></div>
  <div class="brand">
    <img class="icon" src="/app-icon.png" alt="" width="64" height="64" />
    <h1>Fix My Takeout</h1>
    <p>Organize your cloud photo export</p>
  </div>

  <div class="steps">
    <!-- Step 1: Source -->
    <button class="step-btn" onclick={pickSource} class:filled={$sourcePath}>
      <span class="step-num">1</span>
      <div class="step-content">
        <span class="step-label">Select export folder</span>
        {#if $sourcePath}
          <span class="step-path">{$sourcePath}</span>
        {:else}
          <span class="step-hint">Folder containing your .zip files</span>
        {/if}
      </div>
    </button>

    <!-- Step 2: Output (only visible after source is set) -->
    {#if $sourcePath}
      <button class="step-btn" onclick={pickOutput} class:filled={$outputPath}>
        <span class="step-num">2</span>
        <div class="step-content">
          <span class="step-label">Choose output folder</span>
          {#if $outputPath}
            <span class="step-path">{$outputPath}</span>
          {:else}
            <span class="step-hint">Where to save your organized library</span>
          {/if}
        </div>
      </button>
    {/if}

    {#if $outputPath && diskInfo}
      <div class="space-info" class:warn={!spaceOk}>
        {#if spaceOk}
          Sufficient disk space available ({fmt(diskInfo.available_bytes)} free)
        {:else}
          Low disk space — only {fmt(diskInfo.available_bytes)} available
        {/if}
      </div>
    {/if}

    {#if sessionInfo?.exists}
      <div class="resume-info">
        Previous session found — {sessionInfo.zips_done} of {sessionInfo.zips_total} archives processed
      </div>
    {/if}

    {#if error}
      <div class="error-info">{error}</div>
    {/if}

    <!-- Start button (only visible when both are set) -->
    {#if ready}
      <button
        class="step-btn start"
        onclick={handleStart}
        disabled={starting}
      >
        <span class="step-num">
          {#if starting}
            <svg class="spin" width="16" height="16" viewBox="0 0 16 16"><circle cx="8" cy="8" r="6" stroke="white" stroke-width="1.5" fill="none" stroke-dasharray="20 20" stroke-linecap="round"/></svg>
          {:else}
            <svg width="16" height="16" viewBox="0 0 16 16"><path d="M6 4l6 4-6 4V4z" fill="white"/></svg>
          {/if}
        </span>
        <div class="step-content">
          <span class="step-label start-label">
            {starting ? 'Starting...' : sessionInfo?.exists ? 'Resume processing' : 'Start organizing'}
          </span>
        </div>
      </button>
    {/if}
  </div>

  <p class="footnote">Supports iCloud and Google Takeout exports</p>
</div>

<style>
  .welcome {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    padding: 48px 40px;
    padding-top: 56px;
    gap: 32px;
    user-select: none;
    position: relative;
  }

  .drag-region {
    position: absolute; top: 0; left: 0; right: 0; height: 28px;
    z-index: 50; -webkit-app-region: drag;
  }

  .brand { text-align: center; }
  .icon { display: block; margin: 0 auto 12px; border-radius: 14px; }
  h1 {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.5px;
    margin: 0;
    color: var(--text);
  }
  .brand p {
    color: var(--secondary);
    font-size: 14px;
    margin: 4px 0 0;
  }

  .steps {
    width: 100%;
    max-width: 420px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .step-btn {
    display: flex;
    align-items: center;
    gap: 14px;
    width: 100%;
    padding: 14px 18px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--surface);
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    transition: border-color 0.2s, background 0.2s, transform 0.1s;
  }
  .step-btn:hover { border-color: var(--accent); background: var(--accent-faint); }
  .step-btn:active { transform: scale(0.99); }
  .step-btn.filled { border-color: var(--accent-light); }

  .step-num {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--accent-light);
    color: var(--accent);
    font-size: 12px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .step-btn.filled .step-num {
    background: var(--accent);
    color: white;
  }

  .step-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .step-label {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }
  .step-path {
    font-size: 12px;
    color: var(--secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .step-hint {
    font-size: 12px;
    color: var(--secondary);
  }

  .step-btn.start {
    background: var(--accent);
    border-color: var(--accent);
    margin-top: 4px;
  }
  .step-btn.start:hover { background: var(--accent-hover); border-color: var(--accent-hover); }
  .step-btn.start:disabled { opacity: 0.6; cursor: default; transform: none; }
  .step-btn.start .step-num {
    background: rgba(255,255,255,0.2);
    color: white;
  }
  .start-label { color: white !important; }

  .space-info, .resume-info, .error-info {
    font-size: 12px;
    padding: 8px 14px;
    border-radius: 8px;
  }
  .space-info {
    color: var(--secondary);
    background: var(--accent-faint);
  }
  .space-info.warn {
    color: #c44;
    background: #fff0f0;
  }
  .resume-info {
    color: var(--accent);
    background: var(--accent-faint);
  }
  .error-info {
    color: #c44;
    background: #fff0f0;
  }

  .footnote {
    font-size: 12px;
    color: var(--secondary);
  }

  @keyframes spin { to { transform: rotate(360deg); } }
  .spin { animation: spin 0.8s linear infinite; }
</style>
