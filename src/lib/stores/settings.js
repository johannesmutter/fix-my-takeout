import { writable } from 'svelte/store';

export const sourcePath = writable('');
export const outputPath = writable('');
export const currentScreen = writable('welcome'); // 'welcome' | 'processing' | 'done'
