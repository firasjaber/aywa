import type {
	ModelManagerSnapshot,
	WorkerRuntimeBootstrapStatus
} from '$lib/stores/modelManager';

export type SettingsTab = 'app' | 'history' | 'debug';
export type StatusTone = 'positive' | 'warning' | 'progress' | 'neutral' | 'error';

export type AppStatusView = {
	tone: StatusTone;
	title: string;
	detail: string;
	badge: string | null;
	showHotkey: boolean;
};

const preparingStatuses: WorkerRuntimeBootstrapStatus[] = [
	'checking',
	'installing_python_runtime',
	'upgrading_pip',
	'installing_cuda_torch',
	'installing_worker_requirements',
	'falling_back_to_cpu',
	'smoke_testing'
];

export const valueOrFallback = (
	value: string | null | undefined,
	fallback = 'Not available'
) => value?.trim() || fallback;

export const modelLabel = (snapshot: ModelManagerSnapshot, modelId: string | null) =>
	snapshot.models.find((model) => model.modelId === modelId)?.label ?? 'Not available';

export const executionDeviceLabel = (snapshot: ModelManagerSnapshot) => {
	const device = snapshot.liveExecutionDevice;
	if (device === 'gpu') return 'NVIDIA GPU';
	if (device === 'cpu_fallback') return 'CPU';
	return 'Not detected';
};

export const selectAppStatus = (
	snapshot: ModelManagerSnapshot
): AppStatusView => {
	const failedModel = snapshot.readyForTranscription
		? undefined
		: snapshot.models.find((model) => model.errorMessage !== null);
	if (
		!snapshot.readyForTranscription &&
		(snapshot.workerRuntimeBootstrap.status === 'error' || failedModel)
	) {
		return {
			tone: 'error',
			title: 'Needs attention',
			detail: failedModel
				? `${failedModel.label} could not be prepared.`
				: 'The transcription runtime could not be prepared.',
			badge: null,
			showHotkey: false
		};
	}

	const downloading = snapshot.models.find((model) => model.status === 'Downloading');
	if (downloading) {
		return {
			tone: 'progress',
			title: `Downloading ${downloading.label}`,
			detail: 'Installing the model on this device.',
			badge: null,
			showHotkey: false
		};
	}

	const activatingModel = snapshot.models.find((model) => model.status === 'Loading');
	if (activatingModel) {
		return {
			tone: 'progress',
			title: `Activating ${activatingModel.label}`,
			detail: 'This model will become active when it is ready.',
			badge: null,
			showHotkey: false
		};
	}

	if (preparingStatuses.includes(snapshot.workerRuntimeBootstrap.status)) {
		return {
			tone: 'progress',
			title: snapshot.loadedModelId
				? `Preparing ${modelLabel(snapshot, snapshot.loadedModelId)}`
				: 'Preparing transcription',
			detail: 'Aywa is getting the local runtime ready.',
			badge: null,
			showHotkey: false
		};
	}

	if (snapshot.readyForTranscription) {
		if (snapshot.liveExecutionDevice === 'cpu_fallback') {
			return {
				tone: 'warning',
				title: 'Ready — CPU mode',
				detail:
					'Dictation may be slower. NVIDIA users can set up CUDA for faster transcription.',
				badge: 'CPU',
				showHotkey: true
			};
		}

		if (snapshot.liveExecutionDevice === 'gpu') {
			return {
				tone: 'positive',
				title: 'Ready to dictate',
				detail: 'NVIDIA GPU acceleration is active.',
				badge: 'GPU',
				showHotkey: true
			};
		}

		return {
			tone: 'positive',
			title: 'Ready to dictate',
			detail: 'Transcription is ready.',
			badge: null,
			showHotkey: true
		};
	}

	return {
		tone: 'neutral',
		title: 'Choose a model to get started',
		detail: 'Download a model below, then activate it for dictation.',
		badge: null,
		showHotkey: false
	};
};
