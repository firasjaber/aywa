<script lang="ts">
	import { onMount } from 'svelte';
	import { getAppSettings, setRules, type TranscriptionRule } from '$lib/stores/appSettings';

	let rules = $state<TranscriptionRule[]>([]);
	let confirmed = $state<TranscriptionRule[]>([]);
	let loaded = $state(false);
	let saving = $state(false);
	let error = $state<string | null>(null);
	const dirty = $derived(JSON.stringify(rules) !== JSON.stringify(confirmed));
	const errorText = (cause: unknown) => (cause instanceof Error ? cause.message : String(cause));

	const load = async () => {
		error = null;
		try {
			const settings = await getAppSettings();
			rules = settings.rules.map((rule) => ({ ...rule }));
			confirmed = settings.rules.map((rule) => ({ ...rule }));
			loaded = true;
		} catch (cause) {
			error = `Rules could not be loaded: ${errorText(cause)}`;
		}
	};

	onMount(() => {
		void load();
	});

	const save = async () => {
		if (saving || !dirty) return;
		saving = true;
		error = null;
		try {
			const settings = await setRules(rules);
			rules = settings.rules.map((rule) => ({ ...rule }));
			confirmed = settings.rules.map((rule) => ({ ...rule }));
		} catch (cause) {
			error = `Rules were not saved: ${errorText(cause)}`;
		} finally {
			saving = false;
		}
	};
</script>

<section class="rules" aria-labelledby="rules-heading">
	<header>
		<h1 id="rules-heading">Rules</h1>
		<p>Correct words or phrases in new dictations. Leave “Use instead” empty to remove a match.</p>
		<p class="examples">Examples: <code>Aiwa</code> → <code>Aywa</code>. To remove filler sounds, add <code>um</code> and <code>ah</code> as separate rules with empty replacements.</p>
	</header>

	{#if !loaded}
		<p class="status">Loading rules…</p>
		{#if error}<p class="error" role="alert">{error}</p><button type="button" onclick={() => void load()}>Retry</button>{/if}
	{:else}
		<div class="list" aria-label="Transcription rules">
			<div class="column-headings" aria-hidden="true"><span>Heard as</span><span>Use instead</span><span></span></div>
			{#each rules as rule, index}
				<div class="row">
					<label for={`heard-${index}`} class="small-label">Heard as</label>
					<input id={`heard-${index}`} aria-label={`Heard as, rule ${index + 1}`} bind:value={rule.heardAs} disabled={saving} />
					<label for={`instead-${index}`} class="small-label">Use instead</label>
					<input id={`instead-${index}`} aria-label={`Use instead, rule ${index + 1}`} bind:value={rule.useInstead} disabled={saving} />
					<button type="button" class="remove" aria-label={`Delete rule ${index + 1}`} title="Delete rule" disabled={saving} onclick={() => (rules = rules.filter((_, item) => item !== index))}>
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<path d="M4 7h16M9 4h6m-9 3 1 13h10l1-13M10 11v6m4-6v6" />
						</svg>
					</button>
				</div>
			{:else}
				<p class="status">No rules yet.</p>
			{/each}
		</div>
		<div class="actions">
			<button type="button" disabled={saving} onclick={() => (rules = [...rules, { heardAs: '', useInstead: '' }])}>Add rule</button>
			<button type="button" class="save" disabled={saving || !dirty} onclick={() => void save()}>{saving ? 'Saving…' : 'Save rules'}</button>
		</div>
		{#if error}<p class="error" role="alert">{error}</p>{/if}
	{/if}
</section>

<style>
	.rules {
		display: grid;
		gap: 1rem;
	}

	h1,
	p {
		margin: 0;
	}

	h1 {
		font-size: 18px;
	}

	header p,
	.status {
		margin-top: 0.35rem;
		color: #bfc1bc;
		font-size: 12px;
		line-height: 1.45;
	}

	.examples code {
		color: #e3edca;
	}

	.list {
		display: grid;
		gap: 0.35rem;
	}

	.column-headings,
	.row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto;
		gap: 0.5rem;
		align-items: center;
	}

	.column-headings {
		color: #bfc1bc;
		font-size: 12px;
		font-weight: 650;
	}

	.column-headings span:last-child,
	.remove {
		width: 2rem;
	}

	.small-label {
		display: none;
	}

	input {
		min-width: 0;
		width: 100%;
		box-sizing: border-box;
		border: 1px solid #50534f;
		border-radius: 0.35rem;
		padding: 0.45rem;
		background: #151615;
		color: #f4f4f0;
		font: inherit;
		font-size: 13px;
	}

	button {
		border: 1px solid #646962;
		border-radius: 0.4rem;
		padding: 0.45rem 0.6rem;
		background: #2b2e2a;
		color: #f4f4f0;
		font: inherit;
		font-size: 12px;
		cursor: pointer;
	}

	button:disabled {
		opacity: 0.55;
		cursor: default;
	}

	button:focus-visible,
	input:focus-visible {
		outline: 2px solid #d5f18d;
		outline-offset: 2px;
	}

	.remove {
		display: grid;
		place-items: center;
		padding: 0.4rem;
		border-color: transparent;
		background: transparent;
	}

	.remove:hover:not(:disabled) {
		background: #3d3f3d;
	}

	.remove svg {
		width: 1rem;
		height: 1rem;
	}

	.actions {
		display: flex;
		gap: 0.5rem;
		justify-content: space-between;
	}

	.save {
		background: #3d5030;
		border-color: #6d8948;
	}

	.error {
		color: #ffb7b7;
		font-size: 12px;
	}
</style>
