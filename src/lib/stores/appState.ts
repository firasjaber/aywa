import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { readable } from 'svelte/store';

export type AppState = 'ready' | 'listening' | 'processing' | 'completed' | 'error';

export type AppStatePayload = {
	state: AppState;
	message: string | null;
};

const initialState: AppStatePayload = {
	state: 'ready',
	message: null
};

export const appState = readable<AppStatePayload>(initialState, (set) => {
	let unlisten: UnlistenFn | undefined;

	invoke<AppStatePayload>('get_current_state')
		.then(set)
		.catch(() => set(initialState));

	listen<AppStatePayload>('app-state-changed', (event) => {
		set(event.payload);
	})
		.then((cleanup) => {
			unlisten = cleanup;
		})
		.catch(() => set(initialState));

	return () => {
		unlisten?.();
	};
});
