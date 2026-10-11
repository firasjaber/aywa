import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { readable } from 'svelte/store';

export type UpdateSnapshot = {
	revision: number;
	currentVersion: string;
	availableVersion: string | null;
	status: 'idle' | 'checking' | 'upToDate' | 'available' | 'downloading' | 'installing' | 'error';
	downloadedBytes: number;
	totalBytes: number | null;
	downloadReady: boolean;
	error: string | null;
};

let latest: UpdateSnapshot = {
	revision: 0,
	currentVersion: '…',
	availableVersion: null,
	status: 'idle',
	downloadedBytes: 0,
	totalBytes: null,
	downloadReady: false,
	error: null
};
let publish: ((snapshot: UpdateSnapshot) => void) | undefined;

function accept(snapshot: UpdateSnapshot) {
	if (snapshot.revision < latest.revision) return;
	latest = snapshot;
	publish?.(snapshot);
}

export const appUpdate = readable<UpdateSnapshot>(latest, (set) => {
	publish = set;
	set(latest);
	let active = true;
	let unlisten: UnlistenFn | undefined;
	listen<UpdateSnapshot>('app-update-changed', (event) => {
		if (active) accept(event.payload);
	})
		.then((cleanup) => {
			if (!active) { cleanup(); return; }
			unlisten = cleanup;
			return invoke<UpdateSnapshot>('get_update_status');
		})
		.then((snapshot) => { if (active && snapshot) accept(snapshot); })
		.catch(() => {
			if (active && latest.revision === 0) {
				accept({ ...latest, status: 'error', error: 'Could not load update status. Try again.' });
			}
		});
	return () => {
		active = false;
		publish = undefined;
		unlisten?.();
	};
});

export async function checkForUpdates() {
	accept(await invoke<UpdateSnapshot>('check_for_updates'));
}

export async function installUpdate() {
	accept(await invoke<UpdateSnapshot>('install_update'));
}
