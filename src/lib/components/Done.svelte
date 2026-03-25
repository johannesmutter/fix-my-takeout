<script>
  import { zipStatuses } from '../stores/zipStatus.js';
  import { outputPath, sourcePath, currentScreen } from '../stores/settings.js';
  import { resetProgress } from '../stores/progress.js';
  import { getSummaryStats, openInFinder } from '../tauri.js';

  let stats = $state(null);
  let loading = $state(true);

  $effect(() => { loadStats(); });

  async function loadStats() {
    try { stats = await getSummaryStats(); } catch (_) {}
    finally { loading = false; }
  }

  function n(v) { return v?.toLocaleString() ?? '0'; }

  function startOver() {
    resetProgress();
    $sourcePath = '';
    $outputPath = '';
    $currentScreen = 'welcome';
  }

  let cards = $derived(stats ? [
    ['Total files', stats.total_files, 'hero'],
    ['Photos', stats.photos, ''],
    ['Videos', stats.videos, ''],
    ['Live Photos', stats.live_photos, ''],
    ['Screenshots', stats.screenshots, ''],
    ['RAW', stats.raw_images, ''],
    ['Favourites', stats.favourites, ''],
    ['Hidden', stats.hidden, 'muted'],
    ['Deleted', stats.recently_deleted, 'muted'],
    ['Duplicates', stats.duplicates, 'muted'],
    ['No date', stats.unknown_date, 'muted'],
    ['Albums', stats.albums_count, ''],
  ] : []);

  let maxYear = $derived(
    stats?.files_per_year?.length
      ? Math.max(...stats.files_per_year.map(([,c]) => c))
      : 1
  );

  let safeToDelete = $derived($zipStatuses.filter(z => z.safe_to_delete));
</script>

<div class="done">
  <header>
    <div class="check-icon">
      <svg width="40" height="40" viewBox="0 0 40 40" fill="none">
        <circle cx="20" cy="20" r="20" fill="var(--accent)" opacity="0.12"/>
        <path d="M13 20l5 5 9-9" stroke="var(--accent)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </div>
    <h1>All done</h1>
    <p>Your library has been organized</p>
  </header>

  {#if !loading && stats}
    <div class="cards">
      {#each cards as [label, value, variant]}
        <div class="card" class:hero={variant === 'hero'} class:muted={variant === 'muted'}>
          <span class="val">{n(value)}</span>
          <span class="lbl">{label}</span>
        </div>
      {/each}
    </div>

    {#if stats.files_per_year?.length > 0}
      <div class="chart">
        <h3>Timeline</h3>
        {#each stats.files_per_year as [year, count]}
          <div class="row">
            <span class="yr">{year}</span>
            <div class="track"><div class="fill" style="width:{(count / maxYear) * 100}%"></div></div>
            <span class="cnt">{n(count)}</span>
          </div>
        {/each}
      </div>
    {/if}
  {/if}

  <div class="actions">
    <button class="btn primary" onclick={() => openInFinder($outputPath)}>Open in Finder</button>
    <button class="btn" onclick={() => openInFinder($outputPath + '/catalogue.html')}>Browse catalogue</button>
  </div>

  {#if safeToDelete.length > 0}
    <div class="safe">
      <h4>Safe to delete</h4>
      {#each safeToDelete as z}
        <p>{z.zip_name}</p>
      {/each}
    </div>
  {/if}

  <button class="start-over" onclick={startOver}>
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" style="flex-shrink:0">
      <path d="M2.5 7a4.5 4.5 0 0 1 7.7-3.2L8.5 5.5H13V1L11.3 2.7A6 6 0 0 0 1 7h1.5zm9 0a4.5 4.5 0 0 1-7.7 3.2L5.5 8.5H1V13l1.7-1.7A6 6 0 0 0 13 7h-1.5z" fill="currentColor"/>
    </svg>
    Start over with new export
  </button>
</div>

<style>
  .done {
    padding: 40px 32px 48px;
    padding-top: 56px;
    max-width: 640px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 24px;
    min-height: 100vh;
    overflow-y: auto;
    position: relative;
  }


  header { text-align: center; }
  .check-icon { margin: 0 auto 6px; }
  h1 { font-size: 26px; font-weight: 700; margin: 0; letter-spacing: -0.3px; }
  header p { color: var(--secondary); font-size: 14px; margin: 2px 0 0; }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 6px;
    width: 100%;
  }
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 12px 10px;
    text-align: center;
    transition: border-color 0.15s;
  }
  .card:hover { border-color: var(--accent-light); }
  .card.hero { border-color: var(--accent-light); background: var(--accent-faint); }
  .card.hero .val { color: var(--accent); }
  .card.muted .val { color: var(--secondary); }
  .val { display: block; font-size: 20px; font-weight: 700; font-variant-numeric: tabular-nums; }
  .lbl { font-size: 10px; color: var(--secondary); text-transform: uppercase; letter-spacing: 0.4px; }

  .chart {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 16px;
  }
  .chart h3 {
    font-size: 11px; font-weight: 600; text-transform: uppercase;
    letter-spacing: 0.5px; color: var(--secondary); margin: 0 0 10px;
  }
  .row { display: flex; align-items: center; gap: 8px; margin-bottom: 3px; }
  .yr { width: 40px; text-align: right; font-size: 12px; font-weight: 500; font-variant-numeric: tabular-nums; }
  .track { flex: 1; height: 12px; background: var(--border); border-radius: 6px; overflow: hidden; }
  .fill {
    height: 100%; background: var(--accent); border-radius: 6px;
    min-width: 2px;
    transition: width 0.4s cubic-bezier(0.16, 1, 0.3, 1);
  }
  .cnt { width: 44px; font-size: 11px; color: var(--secondary); font-variant-numeric: tabular-nums; }

  .actions { display: flex; gap: 8px; flex-wrap: wrap; justify-content: center; }
  .btn {
    padding: 9px 22px; border-radius: 10px;
    border: 1px solid var(--border); background: var(--surface);
    font-size: 13px; font-weight: 500; font-family: inherit; cursor: pointer;
    color: var(--text); transition: all 0.15s;
  }
  .btn:hover { border-color: var(--accent); background: var(--accent-faint); }
  .btn:active { transform: scale(0.97); }
  .btn.primary {
    background: var(--accent); color: #fff; border-color: var(--accent);
    font-weight: 600;
  }
  .btn.primary:hover { background: var(--accent-hover); border-color: var(--accent-hover); }

  .safe {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 14px 16px;
  }
  .safe h4 {
    font-size: 11px; font-weight: 600; text-transform: uppercase;
    letter-spacing: 0.5px; color: var(--secondary); margin: 0 0 6px;
  }
  .safe p { font-size: 12px; margin: 2px 0; color: var(--secondary); }

  .start-over {
    display: flex; align-items: center; gap: 6px;
    background: var(--surface); border: 1px solid var(--border);
    font-size: 13px; color: var(--text); font-weight: 500;
    cursor: pointer; font-family: inherit;
    padding: 9px 22px; border-radius: 10px;
    transition: all 0.15s;
  }
  .start-over:hover { border-color: var(--accent); background: var(--accent-faint); }
  .start-over:active { transform: scale(0.97); }
</style>
