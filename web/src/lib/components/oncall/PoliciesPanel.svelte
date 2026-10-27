<script lang="ts">
	/**
	 * Escalation policies: who gets paged, in which order, after how long. A
	 * policy does nothing until an "on-call" notification channel points to it,
	 * so each row says which channels use it and the help explains the hookup.
	 */
	import ClientAssign from '#lib/components/ClientAssign.svelte';
	import { ListOrdered, Plus } from 'lucide-svelte';
	import { deleteEscalationPolicy, type Channel, type EscalationPolicy, type Person, type Schedule } from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Panel, Plate } from '#lib/ui/index.js';
	import PolicyEditor from './PolicyEditor.svelte';
	import { personName } from './helpers';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		policies: EscalationPolicy[];
		schedules: Schedule[];
		people: Person[];
		channels: Channel[];
		onchanged: () => void;
	}
	let { policies, schedules, people, channels, onchanged }: Props = $props();

	let form = $state<null | 'new' | EscalationPolicy>(null);
	let busyId = $state<number | null>(null);
	let actionError = $state<unknown>(null);

	function summary(policy: EscalationPolicy): string[] {
		return policy.steps.map((step, i) => {
			const who = step.targets
				.map((t) => {
					if (t.type === 'schedule') return schedules.find((s) => s.id === t.id)?.name ?? `#${t.id}`;
					if (t.type === 'user') return personName(people, t.id);
					return channels.find((c) => c.id === t.id)?.name ?? `#${t.id}`;
				})
				.join(', ');
			return m.oncall_policy_step_summary({ n: i + 1, who: who || '—', minutes: step.delay_minutes });
		});
	}

	async function remove(policy: EscalationPolicy) {
		busyId = policy.id;
		actionError = null;
		try {
			await deleteEscalationPolicy(policy.id);
			onchanged();
		} catch (cause) {
			actionError = cause;
		} finally {
			busyId = null;
		}
	}
</script>

<Panel title={m.oncall_policies_title()} description={m.oncall_policies_description()}>
	{#snippet aside()}
		{#if auth.isAdmin && form === null}
			<Button variant="secondary" onclick={() => (form = 'new')}><Plus class="size-4" aria-hidden="true" />{m.oncall_policy_new()}</Button>
		{/if}
	{/snippet}

	<div class="grid gap-4">
		<div class="rounded-lg border border-line bg-canvas-deep px-4 py-3 text-[0.8125rem] leading-relaxed text-ink-2">
			<p class="font-semibold text-ink">{m.oncall_policies_help_title()}</p>
			<p class="mt-1">{m.oncall_policies_help_body()}</p>
			<a href="/settings/notifications#notifications-channels" class="mt-1.5 inline-flex min-h-10 items-center font-semibold text-signal-ink hover:underline sm:min-h-0">{m.oncall_policies_help_link()}</a>
		</div>

		{#if actionError}<ErrorNotice error={actionError} title={m.oncall_policy_error_delete()} />{/if}

		{#if form !== null}
			{#key form === 'new' ? 'new' : form.id}
				<PolicyEditor
					policy={form === 'new' ? undefined : form}
					{schedules}
					{people}
					{channels}
					onsaved={() => {
						form = null;
						onchanged();
					}}
					oncancel={() => (form = null)}
				/>
			{/key}
		{/if}

		{#if policies.length === 0}
			<EmptyState icon={ListOrdered} title={m.oncall_policies_empty_title()} description={m.oncall_policies_empty_description()} class="!py-8" />
		{:else}
			<ul class="grid gap-2.5">
				{#each policies as policy (policy.id)}
					{@const users = channels.filter((c) => policy.channels.includes(c.id))}
					<li class="rounded-lg border border-line bg-surface px-4 py-3">
						<div class="flex flex-wrap items-start gap-x-4 gap-y-2">
							<div class="min-w-0 flex-1">
								<div class="flex flex-wrap items-center gap-2">
									<span class="font-semibold text-ink">{policy.name}</span>
									{#if policy.repeat > 0}<Plate tone="ghost" bare label={m.oncall_policy_repeats({ count: policy.repeat })} />{/if}
									<ClientAssign kind="escalation_policy" id={policy.id} />
								</div>
								{#if policy.description}<p class="text-[0.8125rem] text-ink-2">{policy.description}</p>{/if}
								<ol class="mt-1.5 grid gap-0.5 text-[0.8125rem] text-ink-2">
									{#each summary(policy) as line (line)}<li>{line}</li>{/each}
								</ol>
								<p class="mt-2 text-[0.8125rem]">
									{#if policy.channels.length === 0}
										<span class="font-medium text-advisory-ink">{m.oncall_policy_unused()}</span>
									{:else}
										<span class="text-ink-2">{m.oncall_policy_used_by({ channels: (users.length ? users.map((c) => c.name) : policy.channels.map((id) => `#${id}`)).join(', ') })}</span>
									{/if}
								</p>
							</div>
							{#if auth.isAdmin}
								<div class="flex flex-wrap gap-2">
									<Button variant="secondary" size="sm" onclick={() => (form = policy)}>{m.oncall_edit()}</Button>
									<Confirm confirmLabel={m.oncall_delete_confirm()} loading={busyId === policy.id} onconfirm={() => remove(policy)}>{m.oncall_delete()}</Confirm>
								</div>
							{/if}
						</div>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</Panel>
