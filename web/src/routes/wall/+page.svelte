<script lang="ts">
	/**
	 * Wall mode: the room monitor. Homelab keeps the city scene and its
	 * bulletin; Enterprise gets its own very dark board (health river, figures,
	 * open alerts, who is on call). The mode is the instance's, not the display's.
	 */
	import { mode } from '#lib/stores/mode.svelte.js';
	import HomelabWall from '#lib/components/wall/HomelabWall.svelte';
	import EnterpriseWall from '#lib/components/wall/EnterpriseWall.svelte';
</script>

{#if mode.enterprise}
	<EnterpriseWall />
{:else}
	<HomelabWall />
{/if}
