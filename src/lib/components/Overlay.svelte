<script lang="ts">
	import { appState } from '$lib/stores/appState';

	const pulseStates = new Set(['listening']);
</script>

<svelte:head>
	<title>Aywa Overlay</title>
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
		justify-content: center;
		box-sizing: border-box;
		padding: 0.25rem;
		pointer-events: none;
	}

	.pill {
		display: inline-flex;
		box-sizing: border-box;
		align-items: center;
		gap: 0.5rem;
		width: 10rem;
		padding: 0.55rem 0.7rem;
		border: 1px solid rgba(244, 244, 240, 0.16);
		border-radius: 0.6rem;
		background: rgba(32, 33, 32, 0.94);
		box-shadow: 0 8px 20px rgba(0, 0, 0, 0.2);
		backdrop-filter: blur(12px);
		color: #f4f4f0;
	}

	.dot {
		width: 0.55rem;
		height: 0.55rem;
		border-radius: 999px;
		background: #ffb36d;
		box-shadow: 0 0 0 0 rgba(255, 179, 109, 0.45);
	}

	.overlay-shell[data-state='completed'] .dot {
		background: #8fd6a0;
		box-shadow: 0 0 0 0 rgba(143, 214, 160, 0.45);
	}

	.overlay-shell[data-state='error'] .dot {
		background: #ef7a7a;
		box-shadow: 0 0 0 0 rgba(239, 122, 122, 0.45);
	}

	.pulse {
		animation: pulse 1.8s ease-out infinite;
	}

	.label {
		font-size: 0.75rem;
		font-weight: 700;
		letter-spacing: 0.04em;
		text-transform: uppercase;
	}

	@keyframes pulse {
		0% {
			box-shadow: 0 0 0 0 rgba(255, 179, 109, 0.45);
		}

		70% {
			box-shadow: 0 0 0 8px rgba(255, 179, 109, 0);
		}

		100% {
			box-shadow: 0 0 0 0 rgba(255, 179, 109, 0);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.pulse {
			animation: none;
		}
	}
</style>
