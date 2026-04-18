<svelte:head>
	<title>OK Settings</title>
</svelte:head>

<section class="settings-shell">
	<div class="settings-card">
		<p class="eyebrow">OK speech</p>
		<h1>Settings</h1>
		<p class="copy">
			Phase 1 only includes the tray, the overlay, the global hotkey, and a fake
			clipboard flow.
		</p>
		<p class="status">Hold Ctrl+Alt+Space to trigger the listening overlay.</p>
		<p class="hint">More settings will arrive in later phases.</p>
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

	@media (max-width: 640px) {
		.settings-shell {
			padding: 1rem;
		}

		.settings-card {
			padding: 1.5rem;
			border-radius: 1.4rem;
		}
	}
</style>
