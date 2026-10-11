<script lang="ts">
	import { appUpdate, checkForUpdates, installUpdate } from '$lib/stores/appUpdate';
	import { appState } from '$lib/stores/appState';
	import { modelManager } from '$lib/stores/modelManager';
	let actionError = $state<string | null>(null);
	let pending = $state(false);
	const updating = $derived(['checking', 'downloading', 'installing'].includes($appUpdate.status));
	const busy = $derived(
		['listening', 'processing'].includes($appState.state) ||
		$modelManager.models.some((model) => ['Downloading', 'Loading'].includes(model.status)) ||
		!['idle', 'ready', 'error'].includes($modelManager.workerRuntimeBootstrap.status)
	);
	const percent = $derived($appUpdate.totalBytes
		? Math.min(100, Math.floor(100 * $appUpdate.downloadedBytes / $appUpdate.totalBytes)) : null);
	async function act(action: () => Promise<void>) {
		if (pending) return;
		pending = true;
		actionError = null;
		try { await action(); }
		catch (error) { actionError = error instanceof Error ? error.message : String(error); }
		finally { pending = false; }
	}
</script>

<div class="app-update">
	<div class="update-controls">
		<span>Aywa {$appUpdate.currentVersion}</span>
		<button type="button" disabled={updating || pending} onclick={() => act(checkForUpdates)}>
			{$appUpdate.status === 'checking' ? 'Checking…' : 'Check for updates'}
		</button>
	</div>
	<div class="update-status" role="status" aria-live="polite">
		{#if $appUpdate.status === 'downloading'}
			<span>Downloading update{percent === null ? '…' : `: ${percent}%`}</span>
			<progress max="100" value={percent ?? undefined} aria-label="Update download progress"></progress>
		{:else if $appUpdate.status === 'installing'}
			<span>Installing update. Aywa will restart.</span>
		{:else if $appUpdate.availableVersion}
			<div class="update-controls">
				<span>Version {$appUpdate.availableVersion} is available</span>
				<button type="button" disabled={busy || updating || pending} onclick={() => act(installUpdate)}>
					{$appUpdate.downloadReady ? 'Install update' : 'Download and install'}
				</button>
			</div>
			<span>{busy ? 'Finish recording, transcription, or model work first.' : 'Installation closes and restarts Aywa.'}</span>
		{:else if $appUpdate.status === 'upToDate'}
			<span>Aywa is up to date.</span>
		{/if}
		{#if actionError || $appUpdate.error}<span class="update-error">{actionError ?? $appUpdate.error}</span>{/if}
	</div>
</div>

<style>
	.app-update { display: grid; gap: 0.35rem; margin-bottom: 0.8rem; }
	.update-controls { display: flex; flex-wrap: wrap; align-items: center; justify-content: center; gap: 0.5rem 0.8rem; }
	.update-status { display: grid; justify-items: center; gap: 0.35rem; }
	button { border: 1px solid #72776b; border-radius: 0.4rem; background: #262923; color: #d5f18d; padding: 0.35rem 0.65rem; font: inherit; cursor: pointer; }
	button:disabled { opacity: 0.55; cursor: default; }
	button:focus-visible { outline: 2px solid #d5f18d; outline-offset: 3px; }
	progress { width: min(100%, 18rem); accent-color: #d5f18d; }
	.update-error { color: #ffb4ab; }
</style>
