<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<svelte:head>
	<title>Aywa</title>
</svelte:head>

{@render children()}
