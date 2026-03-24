import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import {
  progress,
  currentStage,
  addLogLine,
  isComplete,
  errorMessage,
} from './stores/progress.js';
import { zipStatuses } from './stores/zipStatus.js';

export async function selectFolder(title) {
  const selected = await open({
    directory: true,
    multiple: false,
    title,
  });
  return selected;
}

export async function startProcessing(source, output) {
  return invoke('start_processing', { sourcePath: source, outputPath: output });
}

export async function pauseProcessing() {
  return invoke('pause_processing');
}

export async function resumeProcessing() {
  return invoke('resume_processing');
}

export async function cancelProcessing() {
  return invoke('cancel_processing');
}

export async function getZipStatuses() {
  return invoke('get_zip_statuses');
}

export async function getDiskInfo(path) {
  return invoke('get_disk_info', { path });
}

export async function checkExistingSession(outputPath) {
  return invoke('check_existing_session', { outputPath });
}

export async function getSummaryStats() {
  return invoke('get_summary_stats');
}

export async function openInFinder(path) {
  return invoke('open_in_finder', { path });
}

export async function checkForUpdates() {
  try {
    const { check } = await import('@tauri-apps/plugin-updater');
    const update = await check();
    if (!update) return null;
    return {
      version: update.version,
      body: update.body ?? null,
      downloadAndInstall: () => update.downloadAndInstall(),
    };
  } catch (err) {
    console.error('Update check failed:', err);
    return null;
  }
}

export async function setupListeners() {
  await listen('progress', (event) => {
    const p = event.payload;
    progress.set({
      stage: p.stage,
      zipName: p.zip_name,
      zipIndex: p.zip_index,
      zipTotal: p.zip_total,
      filesProcessed: p.files_processed,
      filesTotal: p.files_total,
      bytesProcessed: p.bytes_processed,
      bytesTotal: p.bytes_total,
      elapsedSecs: p.elapsed_secs,
      message: p.message,
    });
  });

  await listen('stage_changed', (event) => {
    const s = event.payload;
    currentStage.set({ stage: s.stage, zipName: s.zip_name });
  });

  await listen('log_line', (event) => {
    const l = event.payload;
    addLogLine(l.level, l.message);
  });

  await listen('zip_status_changed', async () => {
    try {
      const statuses = await getZipStatuses();
      zipStatuses.set(statuses);
    } catch (e) {
      console.error('Failed to get zip statuses:', e);
    }
  });

  await listen('complete', () => {
    isComplete.set(true);
  });

  await listen('error', (event) => {
    errorMessage.set(event.payload);
  });
}
