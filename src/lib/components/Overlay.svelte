<script lang="ts">
	import { appState } from '$lib/stores/appState';

	const pulseStates = new Set(['listening', 'processing']);
</script>

<svelte:head>
	<title>OK Overlay</title>
</svelte:head>

{#if $appState.message}
	<div class="overlay-shell" data-state={$appState.state}>
		<div class="pill" aria-live="polite">
			<span
				class="dot"
				class:pulse={pulseStates.has($appState.state)}
				aria-hidden="true"
			></span>
			<span class="label">{$appState.message}</span>
		</div>
	</div>
{/if}

<style>
	:global(html),
	:global(body) {
		margin: 0;
		width: 100%;
		height: 100%;
		background: transparent;
		overflow: hidden;
	}

	.overlay-shell {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: flex-end;
		padding: 0.75rem;
		pointer-events: none;
	}

	.pill {
		display: inline-flex;
		align-items: center;
		gap: 0.7rem;
		min-width: 11rem;
		padding: 0.85rem 1.15rem;
		border: 1px solid rgba(255, 255, 255, 0.18);
		border-radius: 999px;
		background: linear-gradient(135deg, rgba(18, 27, 29, 0.9), rgba(33, 48, 52, 0.82));
		box-shadow: 0 16px 32px rgba(4, 10, 11, 0.26);
		backdrop-filter: blur(16px);
		color: #f5f1ea;
	}

	.dot {
		width: 0.7rem;
		height: 0.7rem;
		border-radius: 999px;
		background: #ffb36d;
		box-shadow: 0 0 0 0 rgba(255, 179, 109, 0.45);
	}

	.pulse {
		animation: pulse 1.8s ease-out infinite;
	}

	.label {
		font-size: 0.95rem;
		font-weight: 700;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	@keyframes pulse {
		0% {
			box-shadow: 0 0 0 0 rgba(255, 179, 109, 0.45);
		}

		70% {
			box-shadow: 0 0 0 12px rgba(255, 179, 109, 0);
		}

		100% {
			box-shadow: 0 0 0 0 rgba(255, 179, 109, 0);
		}
	}
</style>
