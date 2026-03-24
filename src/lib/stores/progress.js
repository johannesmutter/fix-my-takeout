import { writable } from 'svelte/store';

export const progress = writable({
  stage: '',
  zipName: '',
  zipIndex: 0,
  zipTotal: 0,
  filesProcessed: 0,
  filesTotal: 0,
  bytesProcessed: 0,
  bytesTotal: 0,
  elapsedSecs: 0,
  message: '',
});

export const currentStage = writable({ stage: '', zipName: '' });
export const logLines = writable([]);
export const isComplete = writable(false);
export const isPaused = writable(false);
export const errorMessage = writable('');

export function addLogLine(level, message) {
  logLines.update(lines => {
    const newLines = [...lines, { level, message, time: new Date().toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', hour12: false }) }];
    if (newLines.length > 200) {
      return newLines.slice(-200);
    }
    return newLines;
  });
}

export function resetProgress() {
  progress.set({
    stage: '',
    zipName: '',
    zipIndex: 0,
    zipTotal: 0,
    filesProcessed: 0,
    filesTotal: 0,
    bytesProcessed: 0,
    bytesTotal: 0,
    elapsedSecs: 0,
    message: '',
  });
  currentStage.set({ stage: '', zipName: '' });
  logLines.set([]);
  isComplete.set(false);
  isPaused.set(false);
  errorMessage.set('');
}
