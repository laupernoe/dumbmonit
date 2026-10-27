<script lang="ts">
	/**
	 * MSP mode: which client an object belongs to, changed in place. Shown to
	 * the MSP administrator only. `shared` offers "every client" (the
	 * instance): a channel, rule, maintenance window, report or status page of
	 * the instance serves all clients.
	 */
	import { assignToClient, getClientAssignments, toApiError, type ClientAssignments, type ClientKind } from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		kind: ClientKind;
		id: number;
		/** The object may belong to the whole instance. */
		shared?: boolean;
		/** Called after a move, to reload what depends on it. */
		onchanged?: () => void;
	}
	let { kind, id, shared = true, onchanged }: Props = $props();

	const KEY: Record<ClientKind, keyof ClientAssignments> = {
		target: 'targets',
		status_page: 'status_pages',
		agent_token: 'agent_tokens',
		channel: 'channels',
		rule: 'rules',
		silence: 'silences',
		report: 'reports'
	};

	const visible = $derived(auth.mspOn && auth.msp?.admin === true && (auth.msp?.clients.length ?? 0) > 0);

	let owner = $state<number | null>(null);
	let busy = $state(false);
	let error = $state<string | null>(null);

	$effect(() => {
		if (!visible) return;
		let live = true;
		sharedAssignments()
			.then((map) => {
				if (live) owner = map[KEY[kind]]?.[String(id)] ?? null;
			})
			.catch(() => {});
		return () => {
			live = false;
		};
	});

	async function change(raw: string) {
		const next = raw === '' ? null : Number(raw);
		busy = true;
		error = null;
		try {
			await assignToClient(kind, [id], next);
			owner = next;
			invalidate();
			onchanged?.();
		} catch (cause) {
			error = toApiError(cause).message;
		} finally {
			busy = false;
		}
	}
</script>

<script lang="ts" module>
	// One read of the assignments for a whole list, not one per row.
	let pending: Promise<ClientAssignments> | null = null;

	function sharedAssignments(): Promise<ClientAssignments> {
		pending ??= getClientAssignments().catch((cause) => {
			pending = null;
			throw cause;
		});
		return pending;
	}

	function invalidate() {
		pending = null;
	}
</script>

{#if visible}
	<div class="grid gap-1">
		<label class="sr-only" for={`assign-${kind}-${id}`}>{m.msp_assign_label()}</label>
		<select
			id={`assign-${kind}-${id}`}
			class="input h-10 max-w-[14rem] text-sm"
			value={owner === null ? '' : String(owner)}
			disabled={busy}
			title={m.msp_assign_label()}
			onchange={(event) => void change(event.currentTarget.value)}
		>
			{#if shared || owner === null}
				<option value="" disabled={!shared}>{m.msp_assign_shared()}</option>
			{/if}
			{#each auth.msp?.clients ?? [] as client (client.id)}
				<option value={String(client.id)}>{client.name}</option>
			{/each}
		</select>
		{#if error}<p class="text-sm font-semibold text-warning" role="alert">{error}</p>{/if}
	</div>
{/if}
