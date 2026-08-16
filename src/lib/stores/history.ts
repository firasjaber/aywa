import { invoke } from '@tauri-apps/api/core';

export type HistoryEntry = {
	id: string;
	createdAtEpochMs: number;
	durationMs: number;
	language: string | null;
	preview: string;
	characterCount: number;
};

export type HistoryPage = { entries: HistoryEntry[]; nextOffset: number | null };
export type HistoryTranscript = { id: string; text: string };

export const listHistory = (offset: number) => invoke<HistoryPage>('list_history', { offset });
export const readHistoryEntry = (id: string) => invoke<HistoryTranscript>('read_history_entry', { id });
export const deleteHistoryEntry = (id: string) => invoke<void>('delete_history_entry', { id });
export const clearHistory = () => invoke<void>('clear_history');
