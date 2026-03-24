<script>
  import { progress, currentStage, logLines, isPaused, isComplete, errorMessage } from '../stores/progress.js';
  import { zipStatuses } from '../stores/zipStatus.js';
  import { currentScreen } from '../stores/settings.js';
  import { pauseProcessing, resumeProcessing, cancelProcessing } from '../tauri.js';

  let showLog = $state(false);
  let showCancel = $state(false);
  let logEl = $state(null);

  const stageInfo = {
    extracting: { label: 'Unpacking', desc: 'Extracting files from archive' },
    cataloging: { label: 'Analyzing', desc: 'Reading metadata and organizing files' },
    organizing: { label: 'Organizing', desc: 'Sorting files by date into folders' },
    dedup:      { label: 'Deduplicating', desc: 'Finding and removing duplicate files' },
    symlinks:   { label: 'Creating views', desc: 'Building album and filter views' },
    report:     { label: 'Finishing', desc: 'Generating summary and catalogue' },
  };

  let stage = $derived(stageInfo[$currentStage.stage] || { label: 'Preparing', desc: 'Getting ready...' });

  let overallPct = $derived(
    $progress.filesTotal > 0
      ? Math.round(($progress.filesProcessed / $progress.filesTotal) * 100)
      : 0
  );

  let bytesLabel = $derived(() => {
    if ($progress.bytesTotal > 0) {
      return `${fmt($progress.bytesProcessed)} / ${fmt($progress.bytesTotal)}`;
    }
    return '';
  });

  // ETA calculation
  let eta = $derived(() => {
    const elapsed = $progress.elapsedSecs;
    if (!elapsed || elapsed < 3 || overallPct <= 0 || overallPct >= 100) return '';
    const totalEstimate = elapsed / (overallPct / 100);
    const remaining = Math.max(0, totalEstimate - elapsed);
    if (remaining < 60) return 'less than a minute remaining';
    if (remaining < 3600) {
      const mins = Math.ceil(remaining / 60);
      return `about ${mins} minute${mins !== 1 ? 's' : ''} remaining`;
    }
    const hrs = Math.floor(remaining / 3600);
    const mins = Math.ceil((remaining % 3600) / 60);
    return `about ${hrs}h ${mins}m remaining`;
  });

  function fmt(b) {
    if (b > 1073741824) return `${(b / 1073741824).toFixed(1)} GB`;
    if (b > 1048576) return `${(b / 1048576).toFixed(0)} MB`;
    if (b > 1024) return `${(b / 1024).toFixed(0)} KB`;
    return `${b} B`;
  }

  function fmtTime(s) {
    if (!s || s < 0) return '0:00';
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = Math.floor(s % 60);
    if (h > 0) return `${h}:${String(m).padStart(2,'0')}:${String(sec).padStart(2,'0')}`;
    return `${m}:${String(sec).padStart(2,'0')}`;
  }

  async function togglePause() {
    if ($isPaused) { await resumeProcessing(); $isPaused = false; }
    else { await pauseProcessing(); $isPaused = true; }
  }

  async function doCancel() {
    await cancelProcessing();
    showCancel = false;
    $currentScreen = 'welcome';
  }

  $effect(() => {
    if ($isComplete) $currentScreen = 'done';
  });

  $effect(() => {
    if (logEl && $logLines.length) logEl.scrollTop = logEl.scrollHeight;
  });

  let zipsDone = $derived($zipStatuses.filter(z => z.status === 'done').length);
</script>

<div class="processing">
  <div class="content">
    <div class="header">
      <h1>{stage.label}</h1>
      {#if $progress.zipTotal > 1}
        <span class="counter">{$progress.zipIndex + 1} of {$progress.zipTotal}</span>
      {/if}
    </div>

    <p class="desc">{stage.desc}</p>

    <div class="bar-outer">
      <div class="bar-fill" class:paused={$isPaused} style="width:{overallPct}%"></div>
    </div>

    <div class="stats">
      <span class="pct">{overallPct}%</span>
      {#if $progress.message}
        <span class="msg">{$progress.message}</span>
      {/if}
      {#if $progress.stage === 'extracting' && $progress.bytesTotal > 0}
        <span class="bytes">{bytesLabel()}</span>
      {/if}
      <button class="elapsed-btn" onclick={() => showLog = !showLog} title={showLog ? 'Hide activity log' : 'Show activity log'}>
        <span>{fmtTime($progress.elapsedSecs)}</span>
        <svg class="chevron" class:open={showLog} width="10" height="10" viewBox="0 0 10 10">
          <path d="M3 4l2 2 2-2" stroke="currentColor" stroke-width="1.2" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
    </div>

    {#if eta()}
      <p class="eta">{eta()}</p>
    {/if}

    {#if showLog}
      <div class="log" bind:this={logEl}>
        {#each $logLines as line}
          <div class="log-line" class:warn={line.level === 'warn'} class:err={line.level === 'error'}>
            <span class="log-t">{line.time}</span>
            <span class="log-m">{line.message}</span>
          </div>
        {/each}
      </div>
    {/if}

    {#if $errorMessage}
      <p class="error">{$errorMessage}</p>
    {/if}

    <div class="controls">
      <button class="ctrl" onclick={togglePause}>
        {$isPaused ? 'Resume' : 'Pause'}
      </button>
      <button class="ctrl cancel" onclick={() => showCancel = true}>Cancel</button>
    </div>
  </div>

  <aside class="sidebar">
    <h4>Archives</h4>
    <div class="sidebar-summary">{zipsDone} of {$zipStatuses.length} processed</div>
    {#each $zipStatuses as z}
      <div class="zip" class:done={z.status === 'done'} class:active={z.status !== 'done' && z.status !== 'pending' && z.status !== 'error'} class:err={z.status === 'error'}>
        <span class="dot">
          {#if z.status === 'done'}
            <svg width="10" height="10" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4.5" fill="var(--accent)"/></svg>
          {:else if z.status === 'error'}
            <svg width="10" height="10" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" stroke="#c44" stroke-width="1" fill="none"/><line x1="3.5" y1="3.5" x2="6.5" y2="6.5" stroke="#c44" stroke-width="1" stroke-linecap="round"/></svg>
          {:else if z.status === 'pending'}
            <svg width="10" height="10" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" stroke="var(--border)" stroke-width="1" fill="none"/></svg>
          {:else}
            <svg width="10" height="10" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" stroke="var(--accent)" stroke-width="1" fill="none" stroke-dasharray="2.5 2.5"/></svg>
          {/if}
        </span>
        <span class="zip-name" title={z.zip_name}>{z.zip_name}</span>
      </div>
    {/each}
  </aside>

  {#if showCancel}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="overlay" onclick={() => showCancel = false}>
      <div class="modal" onclick={(e) => e.stopPropagation()}>
        <p><strong>Cancel processing?</strong></p>
        <p class="modal-sub">Progress is saved and you can resume later.</p>
        <div class="modal-btns">
          <button onclick={() => showCancel = false}>Continue</button>
          <button class="modal-cancel" onclick={doCancel}>Cancel</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .processing { display: flex; height: 100vh; overflow: hidden; position: relative; }

  .content {
    flex: 1;
    padding: 36px 32px 32px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow-y: auto;
  }

  .header { display: flex; align-items: baseline; gap: 10px; }
  h1 { font-size: 22px; font-weight: 700; letter-spacing: -0.3px; margin: 0; }
  .counter { font-size: 13px; color: var(--secondary); font-variant-numeric: tabular-nums; }
  .desc { font-size: 13px; color: var(--secondary); margin: -4px 0 4px; }

  .bar-outer {
    width: 100%; height: 5px; border-radius: 3px;
    background: var(--border); overflow: hidden;
  }
  .bar-fill {
    height: 100%; background: var(--accent); border-radius: 3px;
    transition: width 0.4s cubic-bezier(0.16, 1, 0.3, 1);
  }
  .bar-fill.paused { opacity: 0.4; }

  .stats {
    display: flex; align-items: center; gap: 14px;
    font-size: 12px; font-variant-numeric: tabular-nums;
  }
  .pct { font-weight: 700; font-size: 13px; }
  .msg { color: var(--secondary); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bytes { color: var(--secondary); }

  .elapsed-btn {
    margin-left: auto;
    display: flex; align-items: center; gap: 4px;
    background: none; border: none; cursor: pointer;
    font: inherit; font-size: 12px; color: var(--secondary);
    padding: 2px 4px; border-radius: 4px;
    font-variant-numeric: tabular-nums;
    transition: color 0.15s;
  }
  .elapsed-btn:hover { color: var(--text); }
  .chevron { transition: transform 0.2s ease; }
  .chevron.open { transform: rotate(180deg); }

  .eta { font-size: 12px; color: var(--secondary); margin: -2px 0; }

  .log {
    max-height: 160px; overflow-y: auto;
    border: 1px solid var(--border); border-radius: 8px;
    padding: 10px 14px;
    font-family: ui-monospace, 'SF Mono', Menlo, monospace;
    font-size: 10px; line-height: 1.8;
    animation: slide-down 0.15s ease;
  }
  .log-line { display: flex; gap: 10px; white-space: nowrap; }
  .log-line.warn .log-m { color: #d4830a; }
  .log-line.err .log-m { color: var(--accent); }
  .log-t { color: var(--secondary); flex-shrink: 0; }
  .log-m { overflow: hidden; text-overflow: ellipsis; }

  .error {
    font-size: 12px; color: var(--accent); margin: 0;
    padding: 10px 14px; background: var(--accent-faint); border-radius: 8px;
    border: 1px solid var(--accent-light);
  }

  .controls { display: flex; gap: 8px; }
  .ctrl {
    padding: 7px 22px; border-radius: 8px; border: 1px solid var(--border);
    background: var(--surface); font-size: 13px; font-weight: 500;
    font-family: inherit; cursor: pointer; transition: all 0.15s;
  }
  .ctrl:hover { border-color: var(--accent); }
  .ctrl:active { transform: scale(0.97); }
  .ctrl.cancel { color: var(--accent); border-color: var(--accent-light); }
  .ctrl.cancel:hover { border-color: var(--accent); }

  .sidebar {
    width: 200px;
    padding: 36px 14px 16px;
    border-left: 1px solid var(--border);
    overflow-y: auto;
    flex-shrink: 0;
    height: 100vh;
  }
  .sidebar h4 {
    font-size: 10px; font-weight: 600; text-transform: uppercase;
    letter-spacing: 0.5px; color: var(--secondary); margin: 0 0 2px;
  }
  .sidebar-summary {
    font-size: 11px; color: var(--secondary); margin-bottom: 12px;
    font-variant-numeric: tabular-nums;
  }
  .zip {
    display: flex; align-items: center; gap: 7px; padding: 2px 0;
    font-size: 11px; transition: opacity 0.2s;
  }
  .zip.done { opacity: 0.4; }
  .zip.active { font-weight: 500; }
  .dot {
    width: 10px; height: 10px;
    display: flex; align-items: center; justify-content: center;
    flex-shrink: 0;
  }
  .zip-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.2);
    display: flex; align-items: center; justify-content: center; z-index: 100;
    animation: fade-in 0.15s ease;
  }
  .modal {
    background: var(--bg); border: 1px solid var(--border); border-radius: 14px;
    padding: 24px; max-width: 320px; width: 90%;
    animation: scale-in 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    box-shadow: 0 8px 32px rgba(0,0,0,0.08);
  }
  .modal p { margin: 0; font-size: 14px; }
  .modal-sub { color: var(--secondary); font-size: 13px; margin-top: 4px !important; }
  .modal-btns { display: flex; gap: 8px; justify-content: flex-end; margin-top: 18px; }
  .modal-btns button {
    padding: 7px 18px; border-radius: 8px; border: 1px solid var(--border);
    background: var(--surface); font-size: 13px; font-family: inherit; cursor: pointer;
    transition: all 0.15s;
  }
  .modal-btns button:hover { border-color: var(--accent); }
  .modal-btns .modal-cancel { background: var(--accent); color: #fff; border-color: var(--accent); }
  .modal-btns .modal-cancel:hover { background: var(--accent-hover); }

  @keyframes fade-in { from { opacity: 0; } }
  @keyframes scale-in { from { opacity: 0; transform: scale(0.96); } }
  @keyframes slide-down { from { opacity: 0; max-height: 0; } }
</style>
