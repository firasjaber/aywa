import { invoke } from '@tauri-apps/api/core';

export type AppSettings = {
	version: number;
	saveHistory: boolean;
	hotkey: string;
	rules: TranscriptionRule[];
};

export type TranscriptionRule = {
	heardAs: string;
	useInstead: string;
};

export const getAppSettings = () => invoke<AppSettings>('get_app_settings');
export const setSaveHistory = (saveHistory: boolean) =>
	invoke<AppSettings>('set_save_history', { saveHistory });
export const setHotkey = (hotkey: string) => invoke<AppSettings>('set_hotkey', { hotkey });
export const setRules = (rules: TranscriptionRule[]) => invoke<AppSettings>('set_rules', { rules });
