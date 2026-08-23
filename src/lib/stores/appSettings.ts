import { invoke } from '@tauri-apps/api/core';

export type AppSettings = {
	version: number;
	saveHistory: boolean;
};

export const getAppSettings = () => invoke<AppSettings>('get_app_settings');
export const setSaveHistory = (saveHistory: boolean) =>
	invoke<AppSettings>('set_save_history', { saveHistory });
