<script lang="ts">
	/**
	 * "Probe from": the server itself, or a relay agent of a remote site. Shown
	 * only when there is at least one relay; the value is the relay's id, or
	 * `null` for the server. Used by the imports, where every imported monitor
	 * of a kind a relay knows goes through the chosen one.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { listRelays } from '#lib/api/relay.js';
	import type { RelayAgent } from '#lib/api/types.js';
	import { Field } from '#lib/ui/index.js';

	interface Props {
		value: number | null;
		id: string;
		disabled?: boolean;
	}

	let { value = $bindable(null), id, disabled = false }: Props = $props();

	let relays = $state<RelayAgent[]>([]);
	$effect(() => {
		const controller = new AbortController();
		listRelays(controller.signal)
			.then((list) => (relays = list.filter((r) => r.relay).sort((a, b) => a.name.localeCompare(b.name))))
			.catch(() => (relays = []));
		return () => controller.abort();
	});
</script>

{#if relays.length > 0}
	<Field label={m.deviceform_relaypick_label()} for={id} help={m.deviceform_relaypick_help()}>
		<select {id} class="input" bind:value {disabled}>
			<option value={null}>{m.deviceform_relaypick_server()}</option>
			{#each relays as relay (relay.id)}
				<option value={relay.id}>{relay.site ? m.deviceform_form_relay_site({ name: relay.name, site: relay.site }) : relay.name}</option>
			{/each}
		</select>
	</Field>
{/if}
