<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * "Agent required on the mail server": the box at the top of the MDaemon and
	 * SecurityGateway forms, with the choice of the agent installed on that
	 * server.
	 *
	 * Their queues, sessions, Windows services and disk space can only be read
	 * on the server itself, so the monitor leans on the DumbMonit agent running
	 * there. The box says why, offers the Windows install flow (it opens in a
	 * new tab so this form is kept), and stores the chosen agent in the
	 * collector's `input: agent` setting. A host-name match pre-selects it.
	 */
	import { Download, RefreshCw } from 'lucide-svelte';
	import { listTargets, type CollectorInfo, type CollectorOption, type Target } from '#lib/api/index.js';
	import { Button, Field, Plate } from '#lib/ui/index.js';

	interface Props {
		collector: CollectorInfo;
		option: CollectorOption;
		/** Every known monitor; the agents are picked out here. */
		targets: Target[];
		value: string;
		/** The selection was made from the address, not by the user. */
		guessed?: boolean;
		onchange: (value: string) => void;
	}

	let { collector, option, targets, value, guessed = false, onchange }: Props = $props();

	/** Re-read after installing an agent in the other tab. */
	let fresh = $state<Target[] | null>(null);
	let refreshing = $state(false);
	const agents = $derived(
		(fresh ?? targets).filter((t) => t.kind === 'agent').sort((a, b) => a.name.localeCompare(b.name))
	);

	async function refresh() {
		refreshing = true;
		try {
			fresh = await listTargets();
		} catch {
			// The list stays as it was: the user can retry.
		} finally {
			refreshing = false;
		}
	}

	const product = $derived(collector.kind === 'securitygateway' ? 'SecurityGateway' : 'MDaemon');
	const installHref = $derived(`/targets/new?kind=agent&os=windows&then=${collector.kind}`);
	const chosen = $derived(agents.find((a) => String(a.id) === value) ?? null);
</script>

<section
	class={`rounded-lg border px-3 py-3 ${chosen ? 'border-signal/35 bg-signal-soft' : 'border-advisory/35 bg-advisory-soft'}`}
	aria-labelledby="agent-required-title"
>
	<Plate tone={chosen ? 'signal' : 'advisory'} label={chosen ? m.deviceform_mailagent_plate_linked() : m.deviceform_mailagent_plate_required()} />
	<h3 id="agent-required-title" class="mt-1.5 text-sm font-semibold text-ink">{m.deviceform_mailagent_title()}</h3>
	<p class="mt-1 text-sm leading-relaxed text-ink">{m.deviceform_mailagent_why({ product })}</p>

	<div class="mt-3 grid gap-3">
		<Field label={option.label} for="option-agent" help={option.help}>
			<select id="option-agent" class="input" {value} onchange={(event) => onchange(event.currentTarget.value)}>
				<option value="">{m.deviceform_mailagent_none()}</option>
				{#each agents as agent (agent.id)}
					<option value={String(agent.id)}>{agent.name}</option>
				{/each}
			</select>
		</Field>
		{#if guessed && chosen}
			<p class="text-[0.8125rem] text-ink-2">{m.deviceform_mailagent_guessed({ agent: chosen.name })}</p>
		{:else if !chosen}
			<p class="text-[0.8125rem] font-medium text-ink">
				{agents.length === 0 ? m.deviceform_mailagent_no_agents() : m.deviceform_mailagent_unlinked({ product })}
			</p>
		{/if}
		<div class="flex flex-wrap items-center gap-2">
			<Button variant="secondary" href={installHref} target="_blank" rel="noopener">
				<Download class="size-4" aria-hidden="true" />{m.deviceform_mailagent_install()}
			</Button>
			<Button variant="ghost" onclick={refresh} loading={refreshing}>
				<RefreshCw class="size-4" aria-hidden="true" />{m.deviceform_mailagent_refresh()}
			</Button>
		</div>
	</div>
</section>
