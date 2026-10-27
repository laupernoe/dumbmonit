<script lang="ts">
	/**
	 * Under an unfolded alert: the runbooks that apply (rule, folder, device
	 * type), the remediation actions an operator may run on the alert's device,
	 * the automatic policy armed for it, and the recent runs. Nothing shows
	 * when there is nothing to say.
	 */
	import { onMount } from 'svelte';
	import { BookOpen, Wrench } from 'lucide-svelte';
	import type { AlertRemediation } from '#lib/api/index.js';
	import { getAlertRemediation, runRemediationAction } from '#lib/api/remediation.js';
	import { Confirm, Disclosure, ErrorNotice, Plate } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import Markdown from './Markdown.svelte';
	import RunList from './RunList.svelte';

	interface Props {
		fingerprint: string;
	}

	let { fingerprint }: Props = $props();

	let data = $state<AlertRemediation | null>(null);
	let error = $state<unknown>(null);
	let running = $state<number | null>(null);
	let sent = $state(false);

	async function load() {
		try {
			data = await getAlertRemediation(fingerprint);
			error = null;
		} catch {
			// A stand-in alert (device without rule) has no remediation context.
			data = null;
		}
	}

	async function run(id: number) {
		running = id;
		error = null;
		try {
			await runRemediationAction(id, { fingerprint });
			sent = true;
			await load();
		} catch (cause) {
			error = cause;
		} finally {
			running = null;
		}
	}

	onMount(() => {
		void load();
	});

	const armed = $derived(data?.policies.filter((policy) => policy.enabled && !policy.halted_at) ?? []);
	const visible = $derived(
		!!data && (data.runbooks.length > 0 || data.actions.length > 0 || armed.length > 0 || data.runs.length > 0)
	);
</script>

{#if visible && data}
	<div class="mt-4 space-y-3">
		{#if data.runbooks.length > 0}
			<section aria-label={m.remediation_alert_runbooks()}>
				<h4 class="flex items-center gap-1.5 text-[0.75rem] font-medium uppercase tracking-wide text-ink-3">
					<BookOpen class="size-3.5" aria-hidden="true" />{m.remediation_alert_runbooks()}
				</h4>
				<div class="mt-1.5 space-y-2">
					{#each data.runbooks as runbook (runbook.id)}
						<Disclosure title={runbook.title} open={data.runbooks.length === 1}>
							<div class="px-4 pb-3">
								<Markdown source={runbook.body} />
								<a href={`/runbooks/${runbook.id}`} class="mt-2 inline-flex min-h-10 items-center text-[0.8125rem] text-ink-2 underline underline-offset-2 hover:text-ink">
									{m.remediation_runbooks_open()}
								</a>
							</div>
						</Disclosure>
					{/each}
				</div>
			</section>
		{/if}

		{#if data.actions.length > 0 || armed.length > 0}
			<section aria-label={m.remediation_alert_actions()}>
				<h4 class="flex items-center gap-1.5 text-[0.75rem] font-medium uppercase tracking-wide text-ink-3">
					<Wrench class="size-3.5" aria-hidden="true" />{m.remediation_alert_actions()}
				</h4>
				{#each armed as policy (policy.id)}
					<p class="mt-1.5 flex flex-wrap items-center gap-2 text-ink-2">
						<Plate tone="info" label={m.remediation_policies_state_armed()} />
						{m.remediation_alert_policy_armed({ minutes: String(policy.after_minutes), action: policy.action_name })}
					</p>
				{/each}
				{#if auth.canOperate && data.actions.length > 0}
					<div class="mt-2 flex flex-wrap gap-2">
						{#each data.actions as action (action.id)}
							<Confirm
								variant="secondary"
								confirmLabel={m.remediation_actions_run_confirm()}
								loading={running === action.id}
								disabled={running !== null}
								onconfirm={() => run(action.id)}
								class="min-h-10"
							>
								{action.name}
							</Confirm>
						{/each}
					</div>
					{#if sent}
						<p class="mt-2 text-ink-2" role="status">{m.remediation_actions_run_sent()}</p>
					{/if}
				{/if}
				{#if error}
					<ErrorNotice {error} class="mt-2" />
				{/if}
			</section>
		{/if}

		{#if data.runs.length > 0}
			<section aria-label={m.remediation_alert_history()}>
				<h4 class="text-[0.75rem] font-medium uppercase tracking-wide text-ink-3">{m.remediation_alert_history()}</h4>
				<RunList runs={data.runs} showTarget={false} />
			</section>
		{/if}
	</div>
{/if}
