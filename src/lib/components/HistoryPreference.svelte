<script lang="ts">
	import { onMount } from 'svelte';
	import { getAppSettings, setSaveHistory } from '$lib/stores/appSettings';
	import {
		HISTORY_PREFERENCE_DESCRIPTION,
		beginHistoryPreferenceSave,
		historyPreferenceLoadFailed,
		historyPreferenceLoaded,
		historyPreferenceSaveFailed,
		historyPreferenceSaved,
		initialHistoryPreferenceState
	} from './appSettingsView';

	let state = $state(initialHistoryPreferenceState());
	const errorText = (error: unknown) => (error instanceof Error ? error.message : String(error));

	const loadPreference = async () => {
		state = initialHistoryPreferenceState();
		try {
			const settings = await getAppSettings();
			state = historyPreferenceLoaded(settings.saveHistory);
		} catch (error) {
			state = historyPreferenceLoadFailed(errorText(error));
		}
	};

	onMount(() => {
		void loadPreference();
	});

	const changePreference = async (event: Event) => {
		const saveHistory = (event.currentTarget as HTMLInputElement).checked;
		state = beginHistoryPreferenceSave(state, saveHistory);
		try {
			const settings = await setSaveHistory(saveHistory);
			state = historyPreferenceSaved(state, settings.saveHistory);
		} catch (error) {
			state = historyPreferenceSaveFailed(state, errorText(error));
		}
	};
</script>

<section class="history-preference" aria-labelledby="history-preference-heading">
	<div>
		<h2 id="history-preference-heading">History</h2>
		<p>{HISTORY_PREFERENCE_DESCRIPTION}</p>
	</div>
	<label>
		<input
			type="checkbox"
			checked={state.displayed}
			disabled={!state.loaded || state.saving}
			onchange={changePreference}
		/>
		<span>Save transcriptions to History</span>
	</label>
	{#if state.error}
		<div class="error" role="alert">
			<span>The History preference could not be {state.loaded ? 'saved' : 'loaded'}.</span>
			{#if !state.loaded}
				<button type="button" onclick={() => void loadPreference()}>Retry</button>
			{/if}
		</div>
	{/if}
</section>

<style>
	.history-preference {
		display: grid;
		gap: 0.75rem;
		margin-top: 1.1rem;
		border: 1px solid #50534f;
		border-radius: 0.75rem;
		padding: 0.85rem;
		background: #1b1c1b;
	}

	h2,
	p {
		margin: 0;
	}

	h2 {
		font-size: 14px;
	}

	p {
		margin-top: 0.3rem;
		color: #bfc1bc;
		font-size: 12px;
		line-height: 1.45;
	}

	label {
		display: flex;
		align-items: center;
		gap: 0.55rem;
		color: #f4f4f0;
		font-size: 13px;
		font-weight: 650;
		cursor: pointer;
	}

	input {
		width: 1rem;
		height: 1rem;
		accent-color: #c7f164;
	}

	input:disabled {
		cursor: default;
		opacity: 0.6;
	}

	.error {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: #ffb7b7;
		font-size: 12px;
	}

	.error button {
		border: 0;
		padding: 0;
		background: transparent;
		color: inherit;
		font: inherit;
		font-weight: 700;
		text-decoration: underline;
		cursor: pointer;
	}
</style>
