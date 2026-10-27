<script lang="ts">
	/**
	 * MSP mode: which client the pages show. The MSP administrator picks one
	 * client or all of them; a member of several clients picks among his own.
	 * The choice travels with every request (`x-dumbmonit-client`): the server
	 * narrows the lists, it never widens anyone's access. Hidden when there is
	 * nothing to pick.
	 */
	import { Building2 } from 'lucide-svelte';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** Stretches to the available width (phone strip). */
		wide?: boolean;
		id?: string;
	}
	let { wide = false, id = 'client-switcher' }: Props = $props();

	const clients = $derived(auth.msp?.clients ?? []);
	const admin = $derived(auth.msp?.admin === true);
	const shown = $derived(auth.mspOn && (admin ? clients.length > 0 : clients.length > 1));
	const value = $derived(auth.currentClient === null ? '' : String(auth.currentClient));

	function pick(raw: string) {
		const next = raw === '' ? null : Number(raw);
		if (next === auth.currentClient) return;
		auth.selectClient(next);
	}
</script>

{#if shown}
	<div class={`relative flex items-center ${wide ? 'w-full' : 'max-w-[13rem]'}`}>
		<label for={id} class="sr-only">{m.msp_switcher_label()}</label>
		<Building2 class="pointer-events-none absolute left-2.5 size-4 text-ink-2" aria-hidden="true" />
		<select
			{id}
			class="input h-10 w-full min-w-0 truncate !pl-8 text-sm font-semibold"
			{value}
			title={m.msp_switcher_title()}
			onchange={(event) => pick(event.currentTarget.value)}
		>
			{#if admin}
				<option value="">{m.msp_switcher_all()}</option>
			{/if}
			{#each clients as client (client.id)}
				<option value={String(client.id)}>{client.name}</option>
			{/each}
		</select>
	</div>
{/if}
