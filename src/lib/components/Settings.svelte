<script lang="ts">
	import { onMount } from 'svelte';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import History from './History.svelte';
	import {
		downloadModel,
		loadModel,
		modelManager,
		openDiagnosticsLog,
		previewIssueReport,
		type ModelId,
		type ModelRow
	} from '$lib/stores/modelManager';
	import {
		executionDeviceLabel,
		modelLabel,
		selectAppStatus,
		valueOrFallback,
		type SettingsTab
	} from './settingsView';

	const cudaTroubleshootingUrl =
		'https://github.com/firasjaber/aywa/blob/main/docs/TROUBLESHOOTING.md#cuda-is-not-available';
	const userGuideUrl = 'https://github.com/firasjaber/aywa/blob/main/docs/USER_GUIDE.md';
	const authorWebsiteUrl = 'https://firrj.com';
	const sourceCodeUrl = 'https://github.com/firasjaber/aywa';

	let activeTab = $state<SettingsTab>('app');
	let actionError = $state<string | null>(null);
	let issueReportPreview = $state<string | null>(null);
	let reportLoading = $state(false);
	let reportCopied = $state(false);
	let downloadProgress = $state<Partial<Record<ModelId, number>>>({});
	const activatingModelId = $derived(
		$modelManager.models.find((model) => model.status === 'Loading')?.modelId ?? null
	);

	const appStatus = $derived(selectAppStatus($modelManager));

	onMount(() => {
		const timer = window.setInterval(() => {
			const next = { ...downloadProgress };
			let changed = false;

			for (const model of $modelManager.models) {
				if (model.status === 'Downloading') {
					const current = next[model.modelId] ?? 0;
					const progress = Math.min(90, current + Math.max(1, Math.ceil((90 - current) / 12)));
					if (progress !== current) {
						next[model.modelId] = progress;
						changed = true;
					}
				} else if (model.status === 'Downloaded' && next[model.modelId] !== undefined) {
					if (next[model.modelId] !== 100) {
						next[model.modelId] = 100;
						changed = true;
					} else {
						delete next[model.modelId];
						changed = true;
					}
				} else if (model.status === 'Error' && next[model.modelId] !== undefined) {
					delete next[model.modelId];
					changed = true;
				}
			}

			if (changed) downloadProgress = next;
		}, 700);

		return () => window.clearInterval(timer);
	});

	const errorText = (error: unknown) => (error instanceof Error ? error.message : String(error));

	const beginDownload = async (modelId: ModelId) => {
		actionError = null;
		downloadProgress = { ...downloadProgress, [modelId]: 0 };
		try {
			await downloadModel(modelId);
		} catch (error) {
			const next = { ...downloadProgress };
			delete next[modelId];
			downloadProgress = next;
			actionError = errorText(error);
		}
	};

	const activate = async (modelId: ModelId) => {
		actionError = null;
		try {
			await loadModel(modelId);
		} catch (error) {
			actionError = errorText(error);
		}
	};

	const openLog = async () => {
		actionError = null;
		try {
			await openDiagnosticsLog();
		} catch (error) {
			actionError = errorText(error);
		}
	};

	const openHelp = async (url: string) => {
		actionError = null;
		try {
			await openUrl(url);
		} catch (error) {
			actionError = errorText(error);
		}
	};

	const generateIssueReport = async () => {
		actionError = null;
		reportCopied = false;
		reportLoading = true;
		try {
			issueReportPreview = (await previewIssueReport()).preview;
		} catch (error) {
			actionError = errorText(error);
		} finally {
			reportLoading = false;
		}
	};

	const copyIssueReport = async () => {
		if (!issueReportPreview) return;
		actionError = null;
		reportCopied = false;
		try {
			await navigator.clipboard.writeText(issueReportPreview);
			reportCopied = true;
		} catch (error) {
			actionError = errorText(error);
		}
	};

	const modelStateCopy = (model: ModelRow) => {
		if (model.errorMessage || model.status === 'Error') return 'Needs attention';
		if (activatingModelId === model.modelId || model.status === 'Loading') return 'Preparing…';
		if (model.status === 'Loaded') return `Active · Using ${executionDeviceLabel($modelManager)}`;
		if (downloadProgress[model.modelId] !== undefined) {
			return model.status === 'Downloaded'
				? 'Download complete · 100%'
				: `Downloading · ${downloadProgress[model.modelId]}%`;
		}
		if (model.status === 'Downloaded') return 'Ready to activate';
		if (model.status === 'Downloading') return 'Downloading · 0%';
		return 'Not installed';
	};

	const runtimeCheckCopy = $derived.by(() => {
		const result = $modelManager.runtimeProfile.lastTestResult;
		if (result?.success) return 'Last check passed';
		if (result) {
			return valueOrFallback(result.error, 'Last check failed');
		}

		return valueOrFallback($modelManager.workerRuntimeBootstrap.message);
	});

	const statusIconKind = (tone: typeof appStatus.tone) => {
		if (tone === 'positive') return 'check';
		if (tone === 'warning') return 'warning';
		if (tone === 'error') return 'error';
		if (tone === 'progress') return 'spinner';
		return 'download';
	};

	const statusIcon = $derived(statusIconKind(appStatus.tone));
</script>

<svelte:head>
	<title>Aywa Settings</title>
</svelte:head>

<main class="settings-shell">
	<section class="settings-window" aria-label="Aywa settings">
		<header class="app-header">
			<div class="brand">
				<img src="/logo/png/circular/aywa-circle-32.png" alt="" />
				<strong>Aywa</strong>
			</div>
			<div class="tabs" aria-label="Settings sections" role="tablist">
				<button
					type="button"
					class:active={activeTab === 'app'}
					role="tab"
					aria-selected={activeTab === 'app'}
					aria-controls="app-panel"
					onclick={() => (activeTab = 'app')}>App</button
				>
				<button
					type="button"
					class:active={activeTab === 'history'}
					role="tab"
					aria-selected={activeTab === 'history'}
					aria-controls="history-panel"
					onclick={() => (activeTab = 'history')}>History</button
				>
				<button
					type="button"
					class:active={activeTab === 'debug'}
					role="tab"
					aria-selected={activeTab === 'debug'}
					aria-controls="debug-panel"
					onclick={() => (activeTab = 'debug')}>Debug</button
				>
			</div>
			<div class="header-spacer" aria-hidden="true"></div>
		</header>

		{#if activeTab === 'app'}
			<div class="content" id="app-panel" role="tabpanel">
				<section class="status-card {appStatus.tone}" aria-live="polite">
					<div class="status-heading">
						<div class="status-copy">
							<span class="status-icon" aria-hidden="true">
								{#if statusIcon === 'spinner'}
									<svg class="status-spinner" viewBox="0 0 24 24" aria-hidden="true">
										<circle cx="12" cy="12" r="8" />
									</svg>
								{:else if statusIcon === 'download'}
									<svg viewBox="0 0 24 24" aria-hidden="true">
										<path d="M12 3v11m0 0 4-4m-4 4-4-4M5 19h14" />
									</svg>
								{:else if statusIcon === 'check'}
									<svg viewBox="0 0 24 24" aria-hidden="true">
										<path d="m5 12 4 4L19 6" />
									</svg>
								{:else if statusIcon === 'warning'}
									<svg viewBox="0 0 24 24" aria-hidden="true">
										<path d="M12 4 3.5 19h17L12 4Zm0 5v4m0 3h.01" />
									</svg>
								{:else}
									<svg viewBox="0 0 24 24" aria-hidden="true">
										<circle cx="12" cy="12" r="8" />
										<path d="M12 8v5m0 3h.01" />
									</svg>
								{/if}
							</span>
							<div>
								<h1>{appStatus.title}</h1>
								<p>{appStatus.detail}</p>
								{#if appStatus.tone === 'warning'}
									<a
										href={cudaTroubleshootingUrl}
										onclick={(event) => {
											event.preventDefault();
											openHelp(cudaTroubleshootingUrl);
										}}
										>Read the CUDA setup guide →</a
									>
								{:else if appStatus.tone === 'error'}
									<button class="text-action" type="button" onclick={() => (activeTab = 'debug')}
										>View details in Debug →</button
									>
								{/if}
							</div>
						</div>
						{#if appStatus.badge}<span class="status-badge">{appStatus.badge}</span>{/if}
					</div>

					{#if appStatus.showHotkey}
						<div class="hotkey">
							<span><kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Space</kbd></span>
							<span>Hold, speak, then release.</span>
						</div>
					{/if}
				</section>

				<section class="models-section" aria-labelledby="models-heading">
					<div class="section-heading">
						<h2 id="models-heading">Models</h2>
						<span>Choose a model for dictation</span>
					</div>
					<div class="models-list">
						{#each $modelManager.models as model (model.modelId)}
							<article class="model-row">
								<span
									class:active-model={model.status === 'Loaded'}
									class="model-icon"
									aria-hidden="true"
									>{model.status === 'Loaded' ? '✓' : model.label.charAt(0)}</span
								>
								<div class="model-copy">
									<h3>{model.label}</h3>
									<p class:attention={model.errorMessage || model.status === 'Error'}>
										{modelStateCopy(model)}
									</p>
									{#if model.status === 'Downloading' || downloadProgress[model.modelId] !== undefined}
										<progress
											max="100"
											value={downloadProgress[model.modelId] ?? 0}
											aria-label={`${model.status === 'Downloaded' ? 'Downloaded' : 'Downloading'} ${model.label}`}
										></progress>
									{/if}
								</div>

								{#if activatingModelId === model.modelId || model.status === 'Loading'}
									<span class="model-state">Preparing…</span>
								{:else if model.status === 'NotDownloaded' || model.status === 'Error'}
									<button class="model-action" type="button" onclick={() => beginDownload(model.modelId)}
										>Download</button
									>
								{:else if
									model.status === 'Downloading' || downloadProgress[model.modelId] !== undefined}
									<span class="model-progress" aria-hidden="true"
										>{downloadProgress[model.modelId] ?? 0}%</span
									>
								{:else if model.status === 'Downloaded'}
									<button
										class="model-action"
										type="button"
										disabled={activatingModelId !== null}
										onclick={() => activate(model.modelId)}>Activate</button
									>
								{:else if model.status === 'Loaded'}
									<span class="active-badge">Active</span>
								{/if}
							</article>
						{:else}
							<p class="empty-models">No models are available.</p>
						{/each}
					</div>
				</section>

				{#if actionError}
					<p class="app-action-error" role="status">
						That action could not be completed. <button type="button" onclick={() => (activeTab = 'debug')}
							>View details in Debug.</button
						>
					</p>
				{/if}

				<a
					class="help-link"
					href={userGuideUrl}
					onclick={(event) => {
						event.preventDefault();
						openHelp(userGuideUrl);
					}}
					>Need help setting up Aywa?</a
				>
			</div>
		{:else if activeTab === 'history'}
			<div class="content history-content">
				<History />
			</div>
		{:else}
			<div class="content debug-content" id="debug-panel" role="tabpanel">
				<div class="debug-heading">
					<h1>Diagnostics</h1>
					<p>Read-only details for troubleshooting. Recordings are deleted after transcription.</p>
				</div>

				<dl class="diagnostics-list">
					<div>
						<dt>System tools</dt>
						<dd>
							{valueOrFallback($modelManager.operatingSystem)} ·
							{valueOrFallback($modelManager.runtimePreflight.gpuName, 'not detected')} · Driver
							{valueOrFallback($modelManager.runtimePreflight.driverVersion)}
						</dd>
					</div>
					<div>
						<dt>Acceleration</dt>
						<dd>
							<span class:diagnostic-positive={$modelManager.workerCudaStatus.cudaAvailable}
								>{$modelManager.workerCudaStatus.cudaAvailable
									? 'CUDA available'
									: valueOrFallback(
											$modelManager.workerCudaStatus.reason,
											'CUDA not available'
										)}</span
							>
							· CUDA {valueOrFallback($modelManager.workerCudaStatus.cudaVersion)} · PyTorch
							{valueOrFallback($modelManager.workerCudaStatus.torchVersion)} ·
							{executionDeviceLabel($modelManager)}
						</dd>
					</div>
					<div>
						<dt>Runtime</dt>
						<dd>
							<span class:diagnostic-positive={$modelManager.readyForTranscription}
								>{$modelManager.readyForTranscription ? 'Ready' : 'Not ready'}</span
							>
							· {$modelManager.workerRuntimeBootstrap.status} ·
							{runtimeCheckCopy}
						</dd>
					</div>
					<div>
						<dt>Model</dt>
						<dd>
							Selected: {modelLabel($modelManager, $modelManager.selectedModelId)}
							({$modelManager.selectedModelId}) · Active:
							{modelLabel($modelManager, $modelManager.loadedModelId)}
							{#if $modelManager.loadedModelId}({$modelManager.loadedModelId}){/if}
							{#each $modelManager.models.filter((model) => model.errorMessage) as model}
								<br />{model.label}: {model.errorMessage}
							{/each}
						</dd>
					</div>
				</dl>

				{#if actionError}<p class="debug-error" role="alert">{actionError}</p>{/if}

				<div class="debug-actions">
					<button class="secondary-action" type="button" onclick={openLog}>Open local log</button>
					<button class="primary-action" type="button" onclick={generateIssueReport} disabled={reportLoading}
						>{reportLoading ? 'Preparing reportâ€¦' : 'Preview issue report'}</button
					>
				</div>
				<p class="diagnostics-policy">Logs are local, one per launch, and capped at 1 GB total. Review raw logs before sharing them.</p>
				{#if issueReportPreview}
					<label class="report-preview-label" for="issue-report-preview">Review this report before copying</label>
					<textarea id="issue-report-preview" class="report-preview" readonly value={issueReportPreview}></textarea>
					<div class="debug-actions">
						<button class="primary-action" type="button" onclick={copyIssueReport}>Copy reviewed report</button>
						{#if reportCopied}<span class="diagnostic-positive">Copied</span>{/if}
					</div>
				{/if}
			</div>
		{/if}

		<footer class="settings-footer">
			Built by
			<a
				href={authorWebsiteUrl}
				onclick={(event) => {
					event.preventDefault();
					openHelp(authorWebsiteUrl);
				}}>Firas Jaber</a
			>
			<span aria-hidden="true">&mdash;</span>
			<a
				href={sourceCodeUrl}
				onclick={(event) => {
					event.preventDefault();
					openHelp(sourceCodeUrl);
				}}>View source code</a
			>
		</footer>
	</section>
</main>

<style>
	:global(*) {
		box-sizing: border-box;
	}

	.settings-shell {
		min-height: 100vh;
		background: #111212;
		color: #f4f4f0;
		font-size: 14px;
	}

	.settings-window {
		width: 100%;
		display: flex;
		min-height: 100vh;
		height: 100vh;
		flex-direction: column;
		background: #202120;
	}

	.app-header {
		height: 3.25rem;
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		align-items: center;
		padding: 0 1rem;
		border-bottom: 1px solid #484a48;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 14px;
	}

	.brand img {
		width: 1.25rem;
		height: 1.25rem;
	}

	.tabs {
		display: flex;
		gap: 0.25rem;
		padding: 0.25rem;
		border-radius: 0.5rem;
		background: #151615;
	}

	.tabs button {
		border: 0;
		border-radius: 0.35rem;
		padding: 0.4rem 0.8rem;
		background: transparent;
		color: #c2c4c0;
		font: inherit;
		font-size: 12px;
		font-weight: 650;
		cursor: pointer;
	}

	.tabs button.active {
		background: #3d3f3d;
		color: #fff;
	}

	.content {
		min-height: 0;
		flex: 1;
		overflow-y: auto;
		padding: 1.25rem;
	}

	.settings-footer {
		padding: 0 1.25rem 1.25rem;
		color: #bcbeb9;
		font-size: 12px;
		text-align: center;
	}

	.settings-footer a {
		color: #d5f18d;
		font-weight: 700;
		text-decoration: underline;
		text-underline-offset: 0.22rem;
	}

	.status-card,
	.models-list,
	.diagnostics-list {
		border: 1px solid #50534f;
		border-radius: 0.75rem;
	}

	.status-card {
		padding: 1rem;
	}

	.status-card.positive {
		border-color: #727772;
		background: #292b29;
	}

	.status-card.positive .status-icon {
		background: #c7f164;
		color: #162000;
	}

	.status-card.warning {
		border-color: #947a35;
		background: #372d13;
		color: #fff2c9;
	}

	.status-card.warning .status-icon {
		background: #ffe28c;
		color: #3a2a00;
	}

	.status-card.progress {
		border-color: #5c7180;
		background: #202c33;
	}

	.status-card.progress .status-icon {
		background: #9ccbe5;
		color: #10222c;
	}

	.status-card.error {
		border-color: #8a4d4d;
		background: #351f1f;
		color: #ffd9d9;
	}

	.status-card.error .status-icon {
		background: #ffaaaa;
		color: #351010;
	}

	.status-card.neutral {
		border-color: #5d605d;
		background: #292b29;
	}

	.status-card.neutral .status-icon {
		background: #e9e9e2;
		color: #171817;
	}

	.status-heading,
	.status-copy {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
	}

	.status-heading {
		justify-content: space-between;
	}

	.status-icon {
		display: grid;
		width: 1.75rem;
		height: 1.75rem;
		flex: 0 0 auto;
		place-items: center;
		margin-top: 0.1rem;
		border-radius: 50%;
		background: #9aa09a;
		color: #141514;
	}

	.status-icon svg {
		width: 1rem;
		height: 1rem;
		fill: none;
		stroke: currentColor;
		stroke-linecap: round;
		stroke-linejoin: round;
		stroke-width: 2;
	}

	.status-spinner {
		stroke-dasharray: 38 12;
		transform-origin: center;
		animation: status-spin 0.9s linear infinite;
	}

	@keyframes status-spin {
		to {
			transform: rotate(360deg);
		}
	}

	h1,
	h2,
	h3,
	p,
	dl,
	dd {
		margin: 0;
	}

	.status-card h1 {
		font-size: 17px;
		line-height: 1.3;
	}

	.status-card p {
		margin-top: 0.25rem;
		color: #d4d6d1;
		font-size: 13px;
		line-height: 1.45;
	}

	.status-card.warning p {
		color: #e8d79c;
	}

	.status-card.error p {
		color: #efc1c1;
	}

	.status-card a,
	.text-action {
		display: inline-block;
		margin-top: 0.65rem;
		border: 0;
		padding: 0;
		background: transparent;
		color: inherit;
		font: inherit;
		font-size: 12px;
		font-weight: 700;
		text-decoration: underline;
		text-underline-offset: 0.22rem;
		cursor: pointer;
	}

	.status-badge,
	.active-badge {
		flex: 0 0 auto;
		border-radius: 0.35rem;
		font-size: 11px;
		font-weight: 700;
	}

	.status-badge {
		border: 1px solid #687064;
		background: #1c2019;
		padding: 0.25rem 0.5rem;
		color: #d4ee9d;
	}

	.hotkey {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-top: 1rem;
		border-radius: 0.5rem;
		padding: 0.65rem 0.75rem;
		background: #171817;
		color: #bbbdb9;
		font-size: 12px;
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

	.models-section {
		margin-top: 1.1rem;
	}

	.section-heading {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 0.5rem;
	}

	.section-heading h2 {
		font-size: 14px;
	}

	.section-heading span {
		color: #bfc1bc;
		font-size: 12px;
	}

	.models-list {
		overflow: hidden;
		background: #1b1c1b;
	}

	.model-row {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		min-height: 3.8rem;
		padding: 0.7rem 0.75rem;
	}

	.model-row + .model-row {
		border-top: 1px solid #444643;
	}

	.model-icon {
		display: grid;
		width: 2rem;
		height: 2rem;
		flex: 0 0 auto;
		place-items: center;
		border-radius: 0.5rem;
		background: #323431;
		color: #dedfd9;
		font-size: 12px;
		font-weight: 800;
	}

	.model-icon.active-model {
		background: #48553a;
		color: #defb9b;
	}

	.model-copy {
		min-width: 0;
		flex: 1;
	}

	.model-copy h3 {
		overflow: hidden;
		font-size: 13px;
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.model-copy p {
		margin-top: 0.18rem;
		color: #b4b6b1;
		font-size: 12px;
	}

	.model-copy p.attention {
		color: #f2aaaa;
	}

	.model-copy progress {
		display: block;
		width: min(12rem, 100%);
		height: 0.25rem;
		margin-top: 0.4rem;
		border: 0;
		border-radius: 1rem;
		background: #303b41;
		accent-color: #9ccbe5;
	}

	.model-copy progress::-webkit-progress-bar {
		border-radius: 1rem;
		background: #303b41;
	}

	.model-copy progress::-webkit-progress-value {
		border-radius: 1rem;
		background: #9ccbe5;
	}

	.model-action,
	.secondary-action,
	.primary-action {
		flex: 0 0 auto;
		border: 1px solid #777a74;
		border-radius: 0.4rem;
		padding: 0.45rem 0.75rem;
		background: #30322f;
		color: #f4f4f0;
		font: inherit;
		font-size: 12px;
		font-weight: 700;
		cursor: pointer;
	}

	.model-action:hover,
	.secondary-action:hover {
		background: #3a3c39;
	}

	.model-state,
	.model-progress {
		flex: 0 0 auto;
		color: #c7dce7;
		font-size: 12px;
		font-weight: 650;
	}

	.active-badge {
		padding: 0.3rem 0.55rem;
		background: #3b4430;
		color: #d8f59a;
	}

	.empty-models {
		padding: 1rem;
		color: #b4b6b1;
		font-size: 12px;
	}

	.app-action-error {
		margin-top: 0.75rem;
		color: #efb0b0;
		font-size: 12px;
	}

	.app-action-error button {
		border: 0;
		padding: 0;
		background: transparent;
		color: inherit;
		font: inherit;
		font-weight: 700;
		text-decoration: underline;
		cursor: pointer;
	}

	.help-link {
		display: inline-block;
		margin-top: 1.1rem;
		color: #d5f18d;
		font-size: 12px;
		font-weight: 700;
		text-decoration: underline;
		text-underline-offset: 0.22rem;
	}

	.debug-heading h1 {
		font-size: 16px;
	}

	.debug-heading p {
		margin-top: 0.25rem;
		color: #bcbeb9;
		font-size: 12px;
	}

	.diagnostics-list {
		margin-top: 1rem;
		overflow: hidden;
		background: #1b1c1b;
		font-size: 12px;
	}

	.diagnostics-list > div {
		display: grid;
		grid-template-columns: 6.5rem minmax(0, 1fr);
		gap: 0.75rem;
		padding: 0.8rem;
	}

	.diagnostics-list > div + div {
		border-top: 1px solid #454744;
	}

	.diagnostics-list dt {
		color: #bfc1bc;
		font-weight: 700;
	}

	.diagnostics-list dd {
		min-width: 0;
		color: #e3e4df;
		line-height: 1.45;
		overflow-wrap: anywhere;
	}

	.diagnostic-positive {
		color: #d8f59a;
	}

	.debug-error {
		margin-top: 0.75rem;
		border-left: 2px solid #c46d6d;
		padding-left: 0.65rem;
		color: #f0bcbc;
		font-size: 12px;
		line-height: 1.45;
	}

	.debug-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-top: 1rem;
	}

	.diagnostics-policy,
	.report-preview-label {
		display: block;
		margin-top: 0.85rem;
		color: #bcbeb9;
		font-size: 12px;
		line-height: 1.45;
	}

	.report-preview-label {
		color: #e3e4df;
		font-weight: 700;
	}

	.report-preview {
		display: block;
		width: 100%;
		height: 11rem;
		margin-top: 0.45rem;
		resize: vertical;
		border: 1px solid #50534f;
		border-radius: 0.4rem;
		padding: 0.65rem;
		background: #171817;
		color: #e3e4df;
		font: 12px/1.45 ui-monospace, SFMono-Regular, Consolas, monospace;
	}

	.primary-action {
		border-color: #e9e9e2;
		background: #e9e9e2;
		color: #171817;
	}

	button:disabled {
		cursor: not-allowed;
		opacity: 0.52;
	}

	button:focus-visible,
	a:focus-visible {
		outline: 2px solid #d5f18d;
		outline-offset: 2px;
	}

	@media (max-width: 540px), (max-height: 590px) {
		.app-header {
			height: 3rem;
		}

		.content {
			padding: 1rem;
		}

	}

	@media (prefers-reduced-motion: reduce) {
		.status-spinner {
			animation: none;
		}
	}

	@media (max-width: 420px) {
		.section-heading span,
		.header-spacer {
			display: none;
		}

		.app-header {
			grid-template-columns: 1fr auto;
		}

		.diagnostics-list > div {
			grid-template-columns: 1fr;
			gap: 0.25rem;
		}
	}
</style>
