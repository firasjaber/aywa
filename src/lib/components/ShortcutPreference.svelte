<script lang="ts">
	import { onMount } from 'svelte';
	import { getAppSettings, setHotkey } from '$lib/stores/appSettings';
	import { captureShortcut, shortcutLabels } from './shortcutView';

	const DEFAULT_HOTKEY = 'control+alt+Space';

	let confirmed = $state(DEFAULT_HOTKEY);
	let loaded = $state(false);
	let capturing = $state(false);
	let saving = $state(false);
	let error = $state<string | null>(null);
	const labels = $derived(shortcutLabels(confirmed));

	const errorText = (value: unknown) => (value instanceof Error ? value.message : String(value));

	const loadShortcut = async () => {
		loaded = false;
		error = null;
		try {
			const settings = await getAppSettings();
			confirmed = settings.hotkey;
			loaded = true;
		} catch (loadError) {
			error = `The shortcut could not be loaded: ${errorText(loadError)}`;
		}
	};

	onMount(() => {
		void loadShortcut();
	});

	const apply = async (candidate: string) => {
		saving = true;
		error = null;
		try {
			const settings = await setHotkey(candidate);
			confirmed = settings.hotkey;
			loaded = true;
		} catch (saveError) {
			error = errorText(saveError);
		} finally {
			saving = false;
		}
	};

	const beginCapture = () => {
		capturing = true;
		error = null;
	};

	const handleKeydown = (event: KeyboardEvent) => {
		if (!capturing || saving) return;

		event.preventDefault();
		event.stopPropagation();
		const result = captureShortcut(event);
		if (result.kind === 'wait') return;
		if (result.kind === 'cancel') {
			capturing = false;
			error = null;
			return;
		}
		if (result.kind === 'invalid') {
			error = result.message;
			return;
		}

		capturing = false;
		void apply(result.value);
	};
</script>

<svelte:window onkeydown={handleKeydown} />

<div class:capturing class="hotkey">
	<div class="shortcut-copy">
		{#if capturing}
			<strong class="capture-prompt">Press your shortcut…</strong>
			<span>Include Ctrl, Alt, Shift, or the Windows key. Press Escape to cancel.</span>
		{:else}
			<span class="keycaps" aria-label={`Current shortcut: ${labels.join(' plus ')}`}>
				{#each labels as label, index}
					{#if index > 0}<span class="plus" aria-hidden="true">+</span>{/if}
					<kbd>{label}</kbd>
				{/each}
			</span>
			<span>Hold, speak, then release.</span>
		{/if}
	</div>

	<div class="shortcut-actions">
		{#if !loaded}
			<button type="button" disabled={saving} onclick={() => void loadShortcut()}>Retry</button>
		{:else}
			<button type="button" disabled={saving || capturing} onclick={beginCapture}
				>Change shortcut</button
			>
			<button
				type="button"
				disabled={saving || capturing || confirmed === DEFAULT_HOTKEY}
				onclick={() => void apply(DEFAULT_HOTKEY)}>Reset to default</button
			>
		{/if}
	</div>
</div>

{#if saving}
	<p class="shortcut-status" role="status">Saving shortcut…</p>
{/if}
{#if error}
	<p class="shortcut-error" role="alert">{error}</p>
{/if}

<style>
	.hotkey {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.85rem;
		margin-top: 1rem;
		border: 1px solid transparent;
		border-radius: 0.5rem;
		padding: 0.65rem 0.75rem;
		background: #171817;
		color: #bbbdb9;
		font-size: 12px;
	}

	.hotkey.capturing {
		border-color: #99b852;
		background: #1c2118;
	}

	.shortcut-copy,
	.keycaps,
	.shortcut-actions {
		display: flex;
		align-items: center;
	}

	.shortcut-copy {
		flex-wrap: wrap;
		gap: 0.7rem;
		min-width: 0;
	}

	.keycaps {
		gap: 0.25rem;
		color: #f4f4f0;
	}

	.plus {
		color: #8e918b;
	}

	kbd {
		border: 1px solid #686b66;
		border-radius: 0.25rem;
		padding: 0.12rem 0.35rem;
		background: #303230;
		color: #e3e4df;
		font: inherit;
		font-weight: 700;
	}

	.capture-prompt {
		color: #e4f7bd;
		font-size: 12px;
	}

	.shortcut-actions {
		flex: 0 0 auto;
		gap: 0.45rem;
	}

	button {
		border: 1px solid #5f635b;
		border-radius: 0.35rem;
		padding: 0.35rem 0.55rem;
		background: #292b28;
		color: #e4e5e0;
		font: inherit;
		font-weight: 700;
		cursor: pointer;
	}

	button:hover:not(:disabled) {
		border-color: #8b927f;
		background: #333630;
	}

	button:disabled {
		cursor: default;
		opacity: 0.5;
	}

	button:focus-visible {
		outline: 2px solid #c7f164;
		outline-offset: 2px;
	}

	.shortcut-status,
	.shortcut-error {
		margin: 0.4rem 0 0;
		font-size: 12px;
	}

	.shortcut-status {
		color: #c5c8c0;
	}

	.shortcut-error {
		color: #ffb7b7;
	}

	@media (max-width: 720px) {
		.hotkey {
			align-items: flex-start;
			flex-direction: column;
		}

		.shortcut-actions {
			flex-wrap: wrap;
		}
	}
</style>
