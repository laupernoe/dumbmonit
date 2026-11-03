<script lang="ts">
	/**
	 * Settings → Agents → Enrolment approval: which new machines enter on their
	 * own, the hourly enrolment rate per token, the idle delay before a fleet
	 * token is put to sleep, and the machines waiting for approval (accept or
	 * reject, one by one or in bulk). Administrators only.
	 *
	 * Nothing exists for a waiting machine until it is accepted: no
	 * certificate, no measurement, no device.
	 */
	import { ShieldCheck } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { decideAgentEnrolments, getAgentEnrolment, setAgentEnrolmentSettings } from '#lib/api/agent_enrolment.js';
	import type { AgentEnrolmentOverview, AgentEnrolmentSettings, EnrolmentApprovalMode } from '#lib/api/types.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { Button, Disclosure, DocLink, EmptyState, ErrorNotice, Plate, SelectionBar, Skeleton } from '#lib/ui/index.js';

	let data = $state<AgentEnrolmentOverview | null>(null);
	let loadError = $state<unknown>(null);
	let open = $state(false);

	// The form: edited locally, saved in one go.
	let draft = $state<AgentEnrolmentSettings | null>(null);
	let saving = $state(false);
	let saveError = $state<unknown>(null);
	let saved = $state(false);

	let selected = $state<number[]>([]);
	let deciding = $state<'accept' | 'reject' | null>(null);
	let decideError = $state<unknown>(null);

	function adopt(next: AgentEnrolmentOverview) {
		data = next;
		draft = { ...next.settings };
	}

	async function load(signal?: AbortSignal) {
		try {
			const next = await getAgentEnrolment(signal);
			const first = data === null;
			adopt(next);
			loadError = null;
			// Machines waiting: the section opens by itself, they need a decision.
			if (first && next.pending.length > 0) open = true;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			loadError = cause;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	const MODES: { value: EnrolmentApprovalMode; label: () => string }[] = [
		{ value: 'auto', label: m.settings_agents_enrol_mode_auto },
		{ value: 'manual', label: m.settings_agents_enrol_mode_manual },
		{ value: 'ad_domain', label: m.settings_agents_enrol_mode_ad }
	];

	const SUMMARY: Record<EnrolmentApprovalMode, () => string> = {
		auto: m.settings_agents_enrol_summary_auto,
		manual: m.settings_agents_enrol_summary_manual,
		ad_domain: m.settings_agents_enrol_summary_ad
	};

	const pending = $derived(data?.pending ?? []);
	const summary = $derived(
		data
			? pending.length > 0
				? m.settings_agents_enrol_summary_waiting({ count: pending.length })
				: SUMMARY[data.settings.fleet_approval]()
			: undefined
	);

	const dirty = $derived(
		!!data &&
			!!draft &&
			(draft.fleet_approval !== data.settings.fleet_approval ||
				draft.single_use_approval !== data.settings.single_use_approval ||
				Number(draft.rate_per_hour) !== data.settings.rate_per_hour ||
				Number(draft.dormant_days) !== data.settings.dormant_days)
	);

	async function save() {
		if (!draft || saving) return;
		saving = true;
		saveError = null;
		saved = false;
		try {
			adopt(
				await setAgentEnrolmentSettings({
					...draft,
					rate_per_hour: Math.max(0, Math.round(Number(draft.rate_per_hour) || 0)),
					dormant_days: Math.max(0, Math.round(Number(draft.dormant_days) || 0))
				})
			);
			saved = true;
		} catch (cause) {
			saveError = cause;
		} finally {
			saving = false;
		}
	}

	// A machine that left the list (decided elsewhere, reload) leaves the selection.
	const chosen = $derived(selected.filter((id) => pending.some((p) => p.id === id)));
	const selection = $derived<'none' | 'some' | 'all'>(
		chosen.length === 0 ? 'none' : chosen.length === pending.length ? 'all' : 'some'
	);

	function toggle(id: number, on: boolean) {
		selected = on ? [...chosen.filter((x) => x !== id), id] : chosen.filter((x) => x !== id);
	}

	async function decide(ids: number[], approve: boolean) {
		if (deciding || ids.length === 0) return;
		deciding = approve ? 'accept' : 'reject';
		decideError = null;
		try {
			await decideAgentEnrolments(ids, approve);
			selected = [];
			await load();
		} catch (cause) {
			decideError = cause;
		} finally {
			deciding = null;
		}
	}
</script>

<div class="border-b border-line px-5 py-4">
	<Disclosure title={m.settings_agents_enrol_title()} bind:open {summary}>
		<DocLink page="install/agent-security/" />

		{#if loadError && !data}
			<ErrorNotice error={loadError} title={m.settings_agents_enrol_load_error()} class="mt-3" onretry={() => void load()} />
		{:else if !data || !draft}
			<Skeleton class="mt-3 h-24" />
		{:else}
			<div class="mt-4 grid gap-6">
				<!-- Waiting for approval: first, it is what needs a decision. -->
				<section aria-labelledby="enrol-pending" class="grid gap-3">
					<div class="flex flex-wrap items-center gap-2">
						<h4 id="enrol-pending" class="text-sm font-semibold text-ink">{m.settings_agents_enrol_pending_title()}</h4>
						{#if pending.length > 0}
							<Plate tone="advisory" label={m.settings_agents_enrol_pending_count({ count: pending.length, max: data.max_pending })} />
						{/if}
					</div>

					{#if decideError}
						<ErrorNotice error={decideError} title={m.settings_agents_enrol_decide_error()} />
					{/if}

					{#if pending.length === 0}
						<EmptyState icon={ShieldCheck} title={m.settings_agents_enrol_pending_none()} />
					{:else}
						<label class="inline-flex min-h-10 w-fit cursor-pointer items-center gap-3 text-sm font-semibold text-ink">
							<input
								type="checkbox"
								class="size-5 accent-[var(--c-signal)]"
								checked={selection === 'all'}
								indeterminate={selection === 'some'}
								onchange={(event) => (selected = event.currentTarget.checked ? pending.map((p) => p.id) : [])}
							/>
							{m.settings_agents_enrol_select_all({ count: pending.length })}
						</label>
						<ul class="divide-y divide-line overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface" role="list">
							{#each pending as request (request.id)}
								<li class="flex flex-wrap items-start gap-x-3 gap-y-2 border-l-4 border-l-advisory px-4 py-3">
									<label class="flex min-h-10 min-w-0 flex-1 basis-56 cursor-pointer items-start gap-3 has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-signal">
										<input
											type="checkbox"
											class="mt-0.5 size-5 shrink-0 accent-[var(--c-signal)]"
											aria-label={m.settings_agents_enrol_select_one({ host: request.hostname })}
											checked={chosen.includes(request.id)}
											onchange={(event) => toggle(request.id, event.currentTarget.checked)}
										/>
										<span class="min-w-0 flex-1">
											<span class="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-ink">
												<span class="break-all font-semibold">{request.hostname}</span>
												<Plate tone="advisory" label={m.settings_agents_enrol_waiting_badge()} />
											</span>
											<span class="block break-all text-[0.8125rem] text-ink-2">
												{request.source_ip ? m.settings_agents_enrol_from_ip({ ip: request.source_ip }) : m.settings_agents_enrol_unknown_ip()}
												{#if request.os}· {request.os}{/if}
											</span>
											<span class="block break-all text-[0.8125rem] text-ink-2">
												{m.settings_agents_enrol_via_token({ token: request.token_name ?? '?' })}
												· <time class="tnum" datetime={request.requested_at} title={formatDateTime(request.requested_at)}>{m.settings_agents_enrol_requested({ when: formatRelative(request.requested_at) })}</time>
											</span>
										</span>
									</label>
									<div class="flex shrink-0 flex-wrap gap-2">
										<Button size="sm" variant="primary" disabled={deciding !== null} onclick={() => decide([request.id], true)}>
											{m.settings_agents_enrol_accept()}
										</Button>
										<Button size="sm" variant="secondary" disabled={deciding !== null} onclick={() => decide([request.id], false)}>
											{m.settings_agents_enrol_reject()}
										</Button>
									</div>
								</li>
							{/each}
						</ul>
					{/if}
				</section>

				<!-- The policy. -->
				<section aria-labelledby="enrol-policy" class="grid gap-4">
					<h4 id="enrol-policy" class="text-sm font-semibold text-ink">{m.settings_agents_enrol_policy_title()}</h4>
					<div class="grid gap-4 sm:grid-cols-2">
						<div class="grid content-start gap-1.5">
							<label for="enrol-fleet" class="text-sm font-semibold text-ink">{m.settings_agents_enrol_fleet_label()}</label>
							<select id="enrol-fleet" class="input" disabled={saving} bind:value={draft.fleet_approval}>
								{#each MODES as mode (mode.value)}<option value={mode.value}>{mode.label()}</option>{/each}
							</select>
						</div>
						<div class="grid content-start gap-1.5">
							<label for="enrol-single" class="text-sm font-semibold text-ink">{m.settings_agents_enrol_single_label()}</label>
							<select id="enrol-single" class="input" disabled={saving} bind:value={draft.single_use_approval}>
								{#each MODES as mode (mode.value)}<option value={mode.value}>{mode.label()}</option>{/each}
							</select>
							<p class="max-w-prose text-sm text-ink-2">{m.settings_agents_enrol_single_hint()}</p>
						</div>
						<div class="grid content-start gap-1.5">
							<label for="enrol-rate" class="text-sm font-semibold text-ink">{m.settings_agents_enrol_rate_label()}</label>
							<input id="enrol-rate" type="number" inputmode="numeric" min="0" max="10000" step="1" class="input tnum" disabled={saving} bind:value={draft.rate_per_hour} />
							<p class="max-w-prose text-sm text-ink-2">{m.settings_agents_enrol_rate_hint()}</p>
						</div>
						<div class="grid content-start gap-1.5">
							<label for="enrol-dormant" class="text-sm font-semibold text-ink">{m.settings_agents_enrol_dormant_label()}</label>
							<input id="enrol-dormant" type="number" inputmode="numeric" min="0" max="3650" step="1" class="input tnum" disabled={saving} bind:value={draft.dormant_days} />
						</div>
					</div>
					{#if saveError}
						<ErrorNotice error={saveError} title={m.settings_agents_enrol_save_error()} />
					{/if}
					<div class="flex flex-wrap items-center gap-3">
						<Button variant="primary" disabled={!dirty} loading={saving} onclick={save}>{m.settings_agents_enrol_save()}</Button>
						{#if saved && !dirty}<Plate tone="signal" draw label={m.settings_agents_enrol_saved()} />{/if}
					</div>
				</section>
			</div>
		{/if}
	</Disclosure>
</div>

{#if chosen.length > 0}
	<SelectionBar
		countLabel={m.settings_agents_enrol_selected({ count: chosen.length })}
		selectAllLabel={m.settings_agents_enrol_select_all({ count: pending.length })}
		label={m.settings_agents_enrol_bar_label()}
		exitLabel={m.settings_agents_enrol_bar_done()}
		{selection}
		onselectall={(on) => (selected = on ? pending.map((p) => p.id) : [])}
		onexit={() => (selected = [])}
	>
		<Button size="sm" variant="primary" disabled={deciding !== null} loading={deciding === 'accept'} onclick={() => decide(chosen, true)}>
			{m.settings_agents_enrol_accept_selected()}
		</Button>
		<Button size="sm" variant="secondary" disabled={deciding !== null} loading={deciding === 'reject'} onclick={() => decide(chosen, false)}>
			{m.settings_agents_enrol_reject_selected()}
		</Button>
	</SelectionBar>
{/if}
