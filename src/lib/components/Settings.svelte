<script lang="ts">
	import {
		applyRuntimeProfileNow,
		clearModelCache,
		downloadModel,
		loadModel,
		modelManager,
		redetectRuntimeProfile,
		resetRuntimeProfileToAuto,
		selectModel,
		selectRuntimeProfile,
		setRuntimeSelectionMode,
		testRuntimeProfile,
		unloadModel
	} from '$lib/stores/modelManager';

	const onAction = async (action: () => Promise<unknown>) => {
		await action();
	};

	const runtimeProfile = $derived($modelManager.runtimeProfile);
	const runtimeBootstrapStatus = $derived($modelManager.workerRuntimeBootstrap.status);
	const isRuntimePreparing = $derived(
		!['idle', 'ready', 'error'].includes(runtimeBootstrapStatus)
	);
	const hasActiveRuntimeProfile = $derived(!!runtimeProfile.activeProfileId);
	const canLoadModels = $derived(runtimeBootstrapStatus === 'ready' && hasActiveRuntimeProfile);
	const runtimeStatus = $derived($modelManager.readyForTranscription ? 'Ready' : 'Not ready');
	const runtimeBootstrapCopy = $derived(
		$modelManager.workerRuntimeBootstrap.message ??
			(isRuntimePreparing
				? 'Preparing the Python transcription runtime in the background.'
				: hasActiveRuntimeProfile
					? 'Python transcription runtime is ready.'
					: 'No active runtime profile is available yet.')
	);
	const actualExecutionDeviceLabel = $derived(
		runtimeProfile.actualExecutionDevice === 'gpu'
			? 'GPU'
			: runtimeProfile.actualExecutionDevice === 'cpu_fallback'
				? 'CPU fallback'
				: runtimeProfile.lastTestResult?.device === 'gpu'
					? 'GPU'
					: runtimeProfile.lastTestResult?.device === 'cpu_fallback'
						? 'CPU fallback'
						: 'Not tested'
	);
	const transcriptionDeviceLabel = $derived(
		isRuntimePreparing
			? 'Preparing'
			: actualExecutionDeviceLabel === 'GPU'
				? 'NVIDIA GPU'
				: actualExecutionDeviceLabel === 'CPU fallback'
					? 'CPU fallback'
					: 'Not tested'
	);
	const transcriptionDeviceCopy = $derived(
		isRuntimePreparing
			? 'The app is still checking and preparing the Python transcription runtime.'
			: actualExecutionDeviceLabel === 'GPU'
				? 'The current runtime profile and worker startup are using the GPU.'
				: actualExecutionDeviceLabel === 'CPU fallback'
					? 'The active runtime is currently falling back to CPU execution.'
					: 'The runtime has not completed a successful smoke test yet.'
	);

	const formatTimestamp = (value: number | null | undefined) => {
		if (!value) {
			return 'Not tested';
		}

		return new Date(value).toLocaleString();
	};

	const profileLabel = (profileId: string | null) => {
		if (!profileId) {
			return 'None';
		}

		return runtimeProfile.profiles.find((profile) => profile.profileId === profileId)?.label ?? profileId;
	};
</script>

<svelte:head>
	<title>OK Settings</title>
</svelte:head>

<section class="settings-shell">
	<div class="settings-card">
		<p class="eyebrow">OK speech</p>
		<h1>Models</h1>
		<p class="copy">
			Download, verify, and load one supported Parakeet model at a time.
		</p>

		{#if isRuntimePreparing}
			<p class="warning">
				Preparing transcription runtime in the background.
				{runtimeBootstrapCopy}
			</p>
		{:else if runtimeBootstrapStatus === 'error'}
			<p class="error">
				{$modelManager.workerRuntimeBootstrap.message ?? 'Transcription runtime setup failed.'}
			</p>
		{:else if actualExecutionDeviceLabel === 'CPU fallback'}
			<p class="warning">
				Transcription is using CPU fallback.
				{runtimeProfile.lastError ?? $modelManager.workerCudaStatus.reason ?? 'CUDA is not available to the worker runtime.'}
			</p>
		{/if}

		<div class="meta">
			<p class="status">
				Ready for transcription:
				{$modelManager.readyForTranscription ? 'Yes' : 'No'}
			</p>
			<p class="runtime-status">Runtime: {runtimeStatus}</p>
			<p class="runtime-status">Runtime setup: {runtimeBootstrapStatus}</p>
			<p class="hint">{runtimeBootstrapCopy}</p>
			<p class="runtime-status">Transcription device: {transcriptionDeviceLabel}</p>
			<p class="hint">{transcriptionDeviceCopy}</p>
			{#if isRuntimePreparing}
				<p class="hint">You can keep using the settings UI while this finishes.</p>
			{/if}
			{#if $modelManager.workerCudaStatus.gpuName}
				<p class="hint">Worker GPU: {$modelManager.workerCudaStatus.gpuName}</p>
			{/if}
			{#if $modelManager.workerCudaStatus.torchVersion}
				<p class="hint">
					Torch: {$modelManager.workerCudaStatus.torchVersion}
					{#if $modelManager.workerCudaStatus.cudaVersion}
						(CUDA {$modelManager.workerCudaStatus.cudaVersion})
					{/if}
				</p>
			{/if}
			{#if $modelManager.runtimePreflight.gpuName}
				<p class="hint">System GPU: {$modelManager.runtimePreflight.gpuName}</p>
			{/if}
			<div class="toolbar">
				<button class="secondary" onclick={() => onAction(() => clearModelCache())}>
					Clear Cache
				</button>
			</div>
		</div>

		<section class="runtime-panel">
			<div class="runtime-panel-copy">
				<p class="eyebrow runtime-eyebrow">Advanced runtime</p>
				<h2>Runtime profiles</h2>
				<p class="hint">
					Switch between automatic resolution and a specific runtime profile without leaving Settings.
				</p>
			</div>

			<div class="runtime-grid">
				<p class="runtime-status">Selection mode: {runtimeProfile.selectionMode}</p>
				<p class="runtime-status">Resolved auto profile: {profileLabel(runtimeProfile.resolvedAutoProfileId)}</p>
				<p class="runtime-status">Active profile: {profileLabel(runtimeProfile.activeProfileId)}</p>
				<p class="runtime-status">Last successful: {profileLabel(runtimeProfile.lastSuccessfulProfileId)}</p>
				<p class="runtime-status">Execution device: {actualExecutionDeviceLabel}</p>
				<p class="runtime-status">Last smoke test: {formatTimestamp(runtimeProfile.lastTestResult?.testedAtEpochMs)}</p>
			</div>

			{#if runtimeProfile.lastTestResult}
				<p class:warning={!runtimeProfile.lastTestResult.success} class:status={runtimeProfile.lastTestResult.success}>
					Smoke test {runtimeProfile.lastTestResult.success ? 'passed' : 'failed'} for
					{profileLabel(runtimeProfile.lastTestResult.profileId)} on {formatTimestamp(runtimeProfile.lastTestResult.testedAtEpochMs)}.
					{#if runtimeProfile.lastTestResult.error}
						{runtimeProfile.lastTestResult.error}
					{/if}
				</p>
			{/if}

			{#if runtimeProfile.lastError}
				<p class="error">{runtimeProfile.lastError}</p>
			{/if}

			<div class="pill-row">
				<button
					class:secondary={runtimeProfile.selectionMode !== 'auto'}
					disabled={isRuntimePreparing}
					onclick={() => onAction(() => setRuntimeSelectionMode('auto'))}
				>
					Auto
				</button>
				<button
					class:secondary={runtimeProfile.selectionMode !== 'manual'}
					disabled={isRuntimePreparing}
					onclick={() => onAction(() => setRuntimeSelectionMode('manual'))}
				>
					Manual
				</button>
			</div>

			<div class="runtime-controls">
				<label class="picker-label" for="runtime-profile-picker">Profile</label>
				<select
					id="runtime-profile-picker"
					disabled={runtimeProfile.selectionMode !== 'manual' || isRuntimePreparing}
					value={runtimeProfile.selectedProfileId ?? ''}
					onchange={(event) =>
						onAction(() => selectRuntimeProfile((event.currentTarget as HTMLSelectElement).value))}
				>
					<option value="" disabled>Select a profile</option>
					{#each runtimeProfile.profiles as profile}
						<option value={profile.profileId}>
							{profile.label} ({profile.stability})
						</option>
					{/each}
				</select>

				<div class="actions runtime-actions">
					<button
						disabled={isRuntimePreparing || runtimeProfile.selectionMode !== 'manual'}
						onclick={() => onAction(() => applyRuntimeProfileNow())}
					>
						Apply now
					</button>
					<button disabled={isRuntimePreparing} onclick={() => onAction(() => testRuntimeProfile())}>
						Test profile
					</button>
					<button disabled={isRuntimePreparing} onclick={() => onAction(() => redetectRuntimeProfile())}>
						Re-detect
					</button>
					<button disabled={isRuntimePreparing} class="secondary" onclick={() => onAction(() => resetRuntimeProfileToAuto())}>
						Reset to auto
					</button>
				</div>
			</div>
		</section>

		<div class="rows">
			{#each $modelManager.models as model}
				<article class="row">
					<div class="row-copy">
						<h2>{model.label}</h2>
						<p class="status">Status: {model.status}</p>
						{#if model.errorMessage}
							<p class="error">{model.errorMessage}</p>
						{/if}
					</div>

					<div class="actions">
						<button onclick={() => onAction(() => selectModel(model.modelId))} disabled={model.selected}>
							{model.selected ? 'Selected' : 'Select'}
						</button>

						{#if model.status === 'NotDownloaded' || model.status === 'Error'}
							<button onclick={() => onAction(() => downloadModel(model.modelId))}>
								Download
							</button>
						{:else if model.status === 'Downloading'}
							<button disabled>Downloading...</button>
						{:else if model.status === 'Downloaded'}
							{#if canLoadModels}
								<button onclick={() => onAction(() => loadModel(model.modelId))}>Load</button>
							{:else if runtimeBootstrapStatus === 'error'}
								<button disabled>Runtime error</button>
							{:else if !hasActiveRuntimeProfile}
								<button disabled>No active profile</button>
							{:else}
								<button disabled>Preparing runtime...</button>
							{/if}
						{:else if model.status === 'Loaded'}
							<button onclick={() => onAction(() => unloadModel())}>Unload</button>
						{:else}
							<button disabled>{model.status}</button>
						{/if}
					</div>
				</article>
			{/each}
		</div>
	</div>
</section>

<style>
	:global(html),
	:global(body) {
		min-height: 100%;
	}

	:global(body) {
		background:
			radial-gradient(circle at top left, rgba(255, 173, 96, 0.26), transparent 34%),
			linear-gradient(160deg, #f5eee4 0%, #efe6d8 48%, #e8ddd2 100%);
	}

	.settings-shell {
		min-height: 100vh;
		display: grid;
		place-items: center;
		padding: 2rem;
	}

	.settings-card {
		width: min(100%, 44rem);
		padding: 2.25rem;
		border: 1px solid rgba(38, 46, 45, 0.08);
		border-radius: 1.75rem;
		background: rgba(255, 252, 247, 0.88);
		box-shadow: 0 24px 60px rgba(37, 31, 21, 0.14);
		backdrop-filter: blur(18px);
	}

	.eyebrow {
		margin: 0 0 0.75rem;
		font-size: 0.75rem;
		font-weight: 700;
		letter-spacing: 0.18em;
		text-transform: uppercase;
		color: #8a5d21;
	}

	.runtime-eyebrow {
		margin-bottom: 0.4rem;
	}

	h1,
	h2 {
		margin: 0;
	}

	h1 {
		font-size: clamp(2.2rem, 5vw, 3.5rem);
		line-height: 0.96;
		letter-spacing: -0.04em;
	}

	h2 {
		font-size: 1.35rem;
	}

	.copy,
	.hint,
	.status,
	.runtime-status,
	.picker-label {
		margin: 0;
		font-size: 1rem;
		line-height: 1.6;
	}

	.copy {
		margin-top: 1rem;
		max-width: 28rem;
		color: #44504c;
	}

	.meta,
	.runtime-grid,
	.runtime-controls {
		margin-top: 1rem;
		display: grid;
		gap: 0.75rem;
	}

	.status {
		margin-top: 1rem;
		padding: 0.85rem 1rem;
		border-radius: 1rem;
		background: rgba(46, 73, 71, 0.08);
		color: #234141;
	}

	.runtime-status {
		color: #44504c;
	}

	.hint {
		margin-top: 0.4rem;
		color: #5b6764;
	}

	.toolbar,
	.pill-row {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
	}

	.runtime-panel {
		margin-top: 1.5rem;
		padding: 1.2rem;
		border: 1px solid rgba(35, 65, 65, 0.12);
		border-radius: 1.2rem;
		background: rgba(255, 255, 255, 0.52);
	}

	.runtime-panel-copy {
		display: grid;
		gap: 0.2rem;
	}

	.warning,
	.error {
		margin: 1rem 0 0;
		padding: 0.85rem 1rem;
		border-radius: 1rem;
	}

	.warning {
		background: rgba(150, 81, 16, 0.1);
		color: #7a4a16;
	}

	.error {
		background: rgba(147, 39, 39, 0.1);
		color: #8a2626;
	}

	.rows {
		margin-top: 1.5rem;
		display: grid;
		gap: 1rem;
	}

	.row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		gap: 1rem;
		align-items: start;
		padding: 1rem;
		border: 1px solid rgba(38, 46, 45, 0.08);
		border-radius: 1.2rem;
		background: rgba(255, 255, 255, 0.55);
	}

	.row-copy h2 {
		font-size: 1.1rem;
	}

	.row-copy .status,
	.row-copy .error {
		margin-top: 0.75rem;
	}

	.actions {
		display: grid;
		gap: 0.75rem;
	}

	.runtime-actions {
		grid-template-columns: repeat(2, minmax(0, 1fr));
	}

	button,
	select {
		min-width: 7rem;
		padding: 0.8rem 1rem;
		border: 0;
		border-radius: 999px;
		font: inherit;
	}

	button {
		background: #234141;
		color: #fffaf3;
		cursor: pointer;
	}

	select {
		border-radius: 1rem;
		background: rgba(35, 65, 65, 0.08);
		color: #234141;
	}

	button:disabled,
	select:disabled {
		opacity: 0.6;
		cursor: default;
	}

	button.secondary {
		background: rgba(35, 65, 65, 0.12);
		color: #234141;
	}

	@media (max-width: 640px) {
		.settings-shell {
			padding: 1rem;
		}

		.settings-card {
			padding: 1.5rem;
			border-radius: 1.4rem;
		}

		.row {
			grid-template-columns: 1fr;
		}

		.actions,
		.runtime-actions {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}

		button,
		select {
			min-width: 0;
		}
	}
</style>
