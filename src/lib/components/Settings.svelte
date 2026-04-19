<script lang="ts">
	import {
		clearModelCache,
		downloadModel,
		loadModel,
		modelManager,
		selectModel,
		unloadModel
	} from '$lib/stores/modelManager';

	const onAction = async (action: () => Promise<unknown>) => {
		await action();
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

		{#if !$modelManager.runtimePreflight.supported}
			<p class="warning">
				Load is disabled:
				{$modelManager.runtimePreflight.reason ?? 'NVIDIA runtime unavailable.'}
			</p>
		{/if}

		<div class="meta">
			<p class="status">
				Ready for transcription:
				{$modelManager.readyForTranscription ? 'Yes' : 'No'}
			</p>
			{#if $modelManager.runtimePreflight.gpuName}
				<p class="hint">GPU: {$modelManager.runtimePreflight.gpuName}</p>
			{/if}
			<div class="toolbar">
				<button class="secondary" onclick={() => onAction(() => clearModelCache())}>
					Clear Cache
				</button>
			</div>
		</div>

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
							<button
								onclick={() => onAction(() => loadModel(model.modelId))}
								disabled={!$modelManager.runtimePreflight.supported}
							>
								Load
							</button>
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
		width: min(100%, 36rem);
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

	h1 {
		margin: 0;
		font-size: clamp(2.2rem, 5vw, 3.5rem);
		line-height: 0.96;
		letter-spacing: -0.04em;
	}

	.copy,
	.hint,
	.status {
		margin: 0;
		font-size: 1rem;
		line-height: 1.6;
	}

	.copy {
		margin-top: 1rem;
		max-width: 28rem;
		color: #44504c;
	}

	.meta {
		margin-top: 1rem;
		display: grid;
		gap: 0.75rem;
	}

	.status {
		margin-top: 1rem;
		padding: 0.85rem 1rem;
		border-radius: 1rem;
	}

	.status {
		background: rgba(46, 73, 71, 0.08);
		color: #234141;
	}

	.hint {
		margin-top: 1.1rem;
		color: #5b6764;
	}

	.toolbar {
		display: flex;
		justify-content: flex-start;
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
		margin: 0;
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

	button {
		min-width: 7rem;
		padding: 0.8rem 1rem;
		border: 0;
		border-radius: 999px;
		background: #234141;
		color: #fffaf3;
		font: inherit;
		cursor: pointer;
	}

	button:disabled {
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

		.actions {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}

		button {
			min-width: 0;
		}
	}
</style>
