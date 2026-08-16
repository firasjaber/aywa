<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { clearHistory, deleteHistoryEntry, listHistory, readHistoryEntry, type HistoryEntry } from '$lib/stores/history';
	import { formatHistoryDuration, formatRelativeHistoryTime, nextSelectedId } from './historyView';

	let entries = $state<HistoryEntry[]>([]);
	let nextOffset = $state<number | null>(0);
	let selectedId = $state<string | null>(null);
	let selectedText = $state<string | null>(null);
	let loading = $state(true);
	let loadingMore = $state(false);
	let error = $state<string | null>(null);
	let detailError = $state<string | null>(null);
	let copyError = $state<string | null>(null);
	let copied = $state(false);
	let sentinel = $state<HTMLDivElement | undefined>(undefined);
	let observer: IntersectionObserver | undefined;
	let refreshTimer: number | undefined;
	let now = $state(Date.now());

	const selectedEntry = $derived(entries.find((entry) => entry.id === selectedId) ?? null);
	const exactTime = (timestamp: number) => new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(timestamp);
	const failureText = (value: unknown) => (value instanceof Error ? value.message : String(value));

	const selectEntry = async (id: string) => {
		selectedId = id;
		selectedText = null;
		detailError = null;
		copyError = null;
		copied = false;
		try {
			const transcript = await readHistoryEntry(id);
			if (selectedId === id) selectedText = transcript.text;
		} catch (cause) {
			if (selectedId === id) detailError = failureText(cause);
		}
	};

	const loadInitial = async () => {
		loading = true;
		error = null;
		selectedId = null;
		selectedText = null;
		try {
			const page = await listHistory(0);
			entries = page.entries;
			nextOffset = page.nextOffset;
			if (page.entries[0]) await selectEntry(page.entries[0].id);
		} catch (cause) {
			error = failureText(cause);
		} finally {
			loading = false;
		}
	};

	const loadMore = async () => {
		if (nextOffset === null || loadingMore) return;
		loadingMore = true;
		try {
			const page = await listHistory(nextOffset);
			entries = [...entries, ...page.entries];
			nextOffset = page.nextOffset;
		} catch (cause) {
			error = failureText(cause);
		} finally {
			loadingMore = false;
		}
	};

	const copyText = async () => {
		if (!selectedText) return;
		try {
			await navigator.clipboard.writeText(selectedText);
			copied = true;
			copyError = null;
		} catch (cause) {
			copyError = `Could not copy text: ${failureText(cause)}`;
		}
	};

	const removeEntry = async (id = selectedId) => {
		if (!id || !window.confirm('Delete this transcription from local History?')) return;
		const ids = entries.map((entry) => entry.id);
		try {
			await deleteHistoryEntry(id);
			entries = entries.filter((entry) => entry.id !== id);
			selectedText = null;
			detailError = null;
			copyError = null;
			const next = nextSelectedId(ids, id);
			selectedId = next;
			if (next) await selectEntry(next);
		} catch (cause) {
			detailError = failureText(cause);
		}
	};

	const clearAll = async () => {
		if (!window.confirm('Clear all local transcription history? This cannot be undone.')) return;
		try {
			await clearHistory();
			entries = [];
			nextOffset = null;
			selectedId = null;
			selectedText = null;
			detailError = null;
			copyError = null;
		} catch (cause) {
			detailError = failureText(cause);
		}
	};

	onMount(() => {
		void loadInitial();
		refreshTimer = window.setInterval(() => (now = Date.now()), 60_000);
		observer = new IntersectionObserver((items) => {
			if (items.some((item) => item.isIntersecting)) void loadMore();
		});
		if (sentinel) observer.observe(sentinel);
	});

	onDestroy(() => {
		observer?.disconnect();
		if (refreshTimer) window.clearInterval(refreshTimer);
		selectedText = null;
	});
</script>

<div class="history" id="history-panel" role="tabpanel" aria-label="Transcription history">
	<header class="history-heading"><div><h1>History</h1><p>Saved locally on this device. Audio recordings are not retained.</p></div></header>
	{#if loading}
		<p class="status">Loading local history…</p>
	{:else if error && entries.length === 0}
		<div class="status error"><p>History could not be loaded.</p><button type="button" onclick={() => void loadInitial()}>Retry</button></div>
	{:else if entries.length === 0}
		<p class="status">Completed transcriptions will appear here automatically.</p>
	{:else}
		<div class="history-layout">
			<aside class="history-sidebar">
				<div class="history-list" aria-label="Saved transcriptions">
					{#each entries as entry (entry.id)}
						<button class:active={entry.id === selectedId} class="history-row" type="button" onclick={() => void selectEntry(entry.id)}>
							<span class="entry-top"><strong>{formatRelativeHistoryTime(entry.createdAtEpochMs, now)}</strong><span>{formatHistoryDuration(entry.durationMs)}</span></span>
							{#if entry.language}<span class="entry-language">{entry.language}</span>{/if}
							<span class="entry-preview">{entry.preview}</span>
						</button>
					{/each}
					<div bind:this={sentinel} class="sentinel"></div>
					{#if loadingMore}<p class="more">Loading more…</p>{/if}
				</div>
				<button class="clear" type="button" onclick={() => void clearAll()}>Clear all history</button>
			</aside>
			<article class="history-detail">
				{#if selectedEntry}
					<header class="detail-header"><div><h2>{formatRelativeHistoryTime(selectedEntry.createdAtEpochMs, now)}</h2><time datetime={new Date(selectedEntry.createdAtEpochMs).toISOString()}>{exactTime(selectedEntry.createdAtEpochMs)} · {formatHistoryDuration(selectedEntry.durationMs)}</time></div><div class="detail-actions"><button type="button" onclick={() => void copyText()} disabled={!selectedText}>Copy text</button><button class="delete" type="button" onclick={() => void removeEntry()}>Delete</button></div></header>
					{#if selectedEntry.language}<p class="language">{selectedEntry.language}</p>{/if}
					{#if detailError}<div class="detail-error"><p>This transcription is unavailable. {detailError}</p><button class="delete" type="button" onclick={() => void removeEntry()}>Delete entry</button></div>{:else if selectedText === null}<p class="status">Loading transcription…</p>{:else}<pre class="transcript">{selectedText}</pre>{/if}
					{#if copied}<p class="copied" role="status">Copied</p>{/if}
					{#if copyError}<p class="detail-error" role="alert">{copyError}</p>{/if}
				{:else}<p class="status">Select a transcription to read it.</p>{/if}
			</article>
		</div>
	{/if}
</div>

<style>
	.history { min-height: 0; height: 100%; display: flex; flex-direction: column; overflow-x: auto; }
	.history-heading { margin-bottom: 1rem; } h1, h2, p { margin: 0; } h1 { font-size: 16px; } h2 { font-size: 15px; }
	.history-heading p, time, .entry-top span, .entry-language, .more { color: #bcbeb9; font-size: 12px; } .history-heading p { margin-top: .25rem; }
	.history-layout { min-width: 29.5625rem; min-height: 0; flex: 1; display: grid; grid-template-columns: 11.5625rem minmax(18rem, 1fr); border: 1px solid #50534f; border-radius: .75rem; overflow: hidden; background: #1b1c1b; }
	.history-sidebar { min-height: 0; display: flex; flex-direction: column; border-right: 1px solid #444643; } .history-list { min-height: 0; flex: 1; overflow-y: auto; }
	.history-row { width: 100%; border: 0; border-bottom: 1px solid #383a38; padding: .625rem; background: transparent; color: #e9e9e5; text-align: left; cursor: pointer; } .history-row:hover, .history-row.active { background: #323531; } .history-row.active { box-shadow: inset 3px 0 #d5f18d; }
	.entry-top { display: flex; justify-content: space-between; gap: .5rem; font-size: 12px; } .entry-language { display: block; margin-top: .2rem; } .entry-preview { display: -webkit-box; overflow: hidden; margin-top: .35rem; color: #c7c9c4; font-size: 12px; line-height: 1.4; line-clamp: 2; -webkit-box-orient: vertical; -webkit-line-clamp: 2; }
	.clear, .detail-actions button, .detail-error button { border: 1px solid #777a74; border-radius: .4rem; padding: .35rem .55rem; background: #30322f; color: #f4f4f0; font: inherit; font-size: 11px; font-weight: 700; cursor: pointer; } .clear { margin: .5rem; color: #ffb7b7; border-color: #874e4e; } button:disabled { cursor: default; opacity: .55; }
	.history-detail { min-width: 0; min-height: 0; display: flex; flex-direction: column; padding: .75rem; } .detail-header { display: flex; align-items: start; justify-content: space-between; gap: .75rem; } .detail-header time { display: block; margin-top: .25rem; } .detail-actions { display: flex; gap: .4rem; } .delete { border-color: #874e4e !important; color: #ffb7b7 !important; } .language { margin-top: .75rem; color: #bcbeb9; font-size: 12px; }
	.transcript { min-height: 0; flex: 1; overflow: auto; margin: .75rem 0 0; padding: .5rem; border: 1px solid #444643; border-radius: .5rem; background: #161716; color: #e9e9e5; font: inherit; line-height: 1.65; white-space: pre-wrap; } .status { color: #bcbeb9; font-size: 13px; } .history-detail .status { margin: auto 0; } .error, .detail-error { color: #ffb7b7; } .detail-error { margin-top: 1rem; } .detail-error button { margin-top: .75rem; } .copied { margin-top: .6rem; color: #d5f18d; font-size: 12px; font-weight: 700; } .more { padding: .5rem .75rem; } .sentinel { height: 1px; }
</style>
