import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { readable } from 'svelte/store';

export type ModelId = 'parakeet-v0.2-en' | 'parakeet-v0.3-multilingual';

export type ModelStatus =
	| 'NotDownloaded'
	| 'Downloading'
	| 'Downloaded'
	| 'Loading'
	| 'Loaded'
	| 'Error';

export type RuntimePreflight = {
	supported: boolean;
	reason: string | null;
	gpuName: string | null;
	driverVersion: string | null;
};

export type ModelRow = {
	modelId: ModelId;
	label: string;
	status: ModelStatus;
	selected: boolean;
	errorMessage: string | null;
};

export type ModelManagerSnapshot = {
	selectedModelId: ModelId;
	loadedModelId: ModelId | null;
	runtimePreflight: RuntimePreflight;
	readyForTranscription: boolean;
	models: ModelRow[];
};

const initialState: ModelManagerSnapshot = {
	selectedModelId: 'parakeet-v0.2-en',
	loadedModelId: null,
	runtimePreflight: { supported: false, reason: null, gpuName: null, driverVersion: null },
	readyForTranscription: false,
	models: []
};

export const modelManager = readable<ModelManagerSnapshot>(initialState, (set) => {
	let unlisten: UnlistenFn | undefined;

	invoke<ModelManagerSnapshot>('list_models').then(set).catch(() => set(initialState));

	listen<ModelManagerSnapshot>('model-manager-changed', (event) => {
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

export const selectModel = (modelId: ModelId) => invoke('select_model', { modelId });
export const downloadModel = (modelId: ModelId) => invoke('download_model', { modelId });
export const loadModel = (modelId: ModelId) => invoke('load_model', { modelId });
export const unloadModel = () => invoke('unload_model');
export const clearModelCache = () => invoke('clear_model_cache');
