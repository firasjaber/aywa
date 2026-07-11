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

export type WorkerCudaStatus = {
	cudaAvailable: boolean;
	reason: string | null;
	gpuName: string | null;
	torchVersion: string | null;
	cudaVersion: string | null;
};

export type WorkerRuntimeBootstrapStatus =
	| 'idle'
	| 'checking'
	| 'installing_python_runtime'
	| 'upgrading_pip'
	| 'installing_cuda_torch'
	| 'installing_worker_requirements'
	| 'falling_back_to_cpu'
	| 'smoke_testing'
	| 'ready'
	| 'error';

export type WorkerRuntimeBootstrap = {
	status: WorkerRuntimeBootstrapStatus;
	message: string | null;
};

export type ModelRow = {
	modelId: ModelId;
	label: string;
	status: ModelStatus;
	selected: boolean;
	errorMessage: string | null;
};

export type RuntimeSelectionMode = 'auto' | 'manual';

export type RuntimeSmokeTestResult = {
	profileId: string;
	success: boolean;
	device: string;
	torchVersion: string | null;
	cudaVersion: string | null;
	gpuName: string | null;
	testedAtEpochMs: number;
	error: string | null;
};

export type RuntimeProfileCatalogOption = {
	profileId: string;
	label: string;
	stability: string;
	autoEligible: boolean;
};

export type RuntimeProfileSnapshot = {
	selectionMode: RuntimeSelectionMode;
	selectedProfileId: string | null;
	resolvedAutoProfileId: string | null;
	activeProfileId: string | null;
	lastSuccessfulProfileId: string | null;
	manifestVersion: string | null;
	actualExecutionDevice: string | null;
	lastTestResult: RuntimeSmokeTestResult | null;
	lastError: string | null;
	profiles: RuntimeProfileCatalogOption[];
};

export type ModelManagerSnapshot = {
	selectedModelId: ModelId;
	loadedModelId: ModelId | null;
	operatingSystem: string;
	runtimePreflight: RuntimePreflight;
	workerCudaStatus: WorkerCudaStatus;
	workerRuntimeBootstrap: WorkerRuntimeBootstrap;
	runtimeProfile: RuntimeProfileSnapshot;
	diagnosticsLogPath: string | null;
	readyForTranscription: boolean;
	models: ModelRow[];
};

const initialState: ModelManagerSnapshot = {
	selectedModelId: 'parakeet-v0.2-en',
	loadedModelId: null,
	operatingSystem: 'Windows',
	runtimePreflight: { supported: false, reason: null, gpuName: null, driverVersion: null },
	workerCudaStatus: {
		cudaAvailable: false,
		reason: null,
		gpuName: null,
		torchVersion: null,
		cudaVersion: null
	},
	workerRuntimeBootstrap: {
		status: 'idle',
		message: null
	},
	runtimeProfile: {
		selectionMode: 'auto',
		selectedProfileId: null,
		resolvedAutoProfileId: null,
		activeProfileId: null,
		lastSuccessfulProfileId: null,
		manifestVersion: null,
		actualExecutionDevice: null,
		lastTestResult: null,
		lastError: null,
		profiles: []
	},
	diagnosticsLogPath: null,
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
export const openDiagnosticsLog = () => invoke<void>('open_diagnostics_log');
export const downloadModel = (modelId: ModelId) => invoke('download_model', { modelId });
export const loadModel = (modelId: ModelId) => invoke('load_model', { modelId });
export const unloadModel = () => invoke('unload_model');
export const clearModelCache = () => invoke('clear_model_cache');
export const setRuntimeSelectionMode = (selectionMode: RuntimeSelectionMode) =>
	invoke('set_runtime_selection_mode', { selectionMode });
export const selectRuntimeProfile = (profileId: string) =>
	invoke('select_runtime_profile', { profileId });
export const applyRuntimeProfileNow = () => invoke('apply_runtime_profile_now');
export const testRuntimeProfile = () => invoke('test_runtime_profile');
export const redetectRuntimeProfile = () => invoke('redetect_runtime_profile');
export const resetRuntimeProfileToAuto = () => invoke('reset_runtime_profile_to_auto');
