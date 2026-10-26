<script lang="ts">
	/**
	 * Settings → Agents → Agent updates: the DumbMonit agent updating its own
	 * binary (not the Docker container `auto_update` policies). Global switches,
	 * the rollout (canaries first), then one card per agent. Administrators only.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { getAgentUpdates, setAgentUpdatePolicy, setAgentUpdateSettings, updateAgentNow, updateAllAgentsNow } from '#lib/api/agent_updates.js';
	import type { AgentUpdateAgent, AgentUpdates, AgentUpdateSettings, AgentUpdateStatus } from '#lib/api/types.js';
	import { formatDateTime } from '#lib/format.js';
	import { Button, Confirm, Disclosure, ErrorNotice, Plate, Skeleton, Toggle, type Tone } from '#lib/ui/index.js';

	let data = $state<AgentUpdates | null>(null);
	let loadError = $state<unknown>(null);
	let error = $state<unknown>(null);
	let busy = $state(false);
	let open = $state(false);
	let canaryInput = $state('0');

	function adopt(next: AgentUpdates) {
		data = next;
		canaryInput = String(next.settings.canary_minutes);
	}

	$effect(() => {
		if (!open || data) return;
		const controller = new AbortController();
		getAgentUpdates(controller.signal)
			.then(adopt)
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				loadError = cause;
			});
		return () => controller.abort();
	});

	async function act(change: () => Promise<AgentUpdates>) {
		busy = true;
		error = null;
		try {
			adopt(await change());
		} catch (cause) {
			error = cause;
			if (data) canaryInput = String(data.settings.canary_minutes);
		} finally {
			busy = false;
		}
	}

	const save = (patch: Partial<AgentUpdateSettings>) => act(() => setAgentUpdateSettings(patch));

	function saveCanary() {
		const value = Math.round(Number(canaryInput));
		if (!Number.isFinite(value) || value < 0 || value > 10080 || value === data?.settings.canary_minutes) {
			if (data) canaryInput = String(data.settings.canary_minutes);
			return;
		}
		void save({ canary_minutes: value });
	}

	const unsignedBuild = $derived(data?.binaries.some((b) => !b.signed) ?? false);

	const TONE: Record<AgentUpdateStatus, Tone> = {
		up_to_date: 'signal',
		pending: 'info',
		updating: 'info',
		failed: 'warning',
		refused: 'advisory',
		disabled: 'ghost',
		manual: 'ghost',
		waiting_window: 'ghost',
		waiting_canary: 'ghost',
		newer: 'ghost',
		legacy: 'ghost',
		container: 'ghost',
		read_only: 'ghost',
		disabled_locally: 'ghost',
		unsupported: 'ghost',
		no_binary: 'ghost'
	};
	const LABEL: Record<AgentUpdateStatus, () => string> = {
		up_to_date: m.settings_agentupdates_status_up_to_date,
		pending: m.settings_agentupdates_status_pending,
		updating: m.settings_agentupdates_status_updating,
		failed: m.settings_agentupdates_status_failed,
		refused: m.settings_agentupdates_status_refused,
		disabled: m.settings_agentupdates_status_disabled,
		manual: m.settings_agentupdates_status_manual,
		waiting_window: m.settings_agentupdates_status_waiting_window,
		waiting_canary: m.settings_agentupdates_status_waiting_canary,
		newer: m.settings_agentupdates_status_newer,
		legacy: m.settings_agentupdates_status_legacy,
		container: m.settings_agentupdates_status_container,
		read_only: m.settings_agentupdates_status_read_only,
		disabled_locally: m.settings_agentupdates_status_disabled_locally,
		unsupported: m.settings_agentupdates_status_unsupported,
		no_binary: m.settings_agentupdates_status_no_binary
	};
	const HINT: Record<AgentUpdateStatus, () => string> = {
		up_to_date: m.settings_agentupdates_hint_up_to_date,
		pending: m.settings_agentupdates_hint_pending,
		updating: m.settings_agentupdates_hint_updating,
		failed: m.settings_agentupdates_hint_failed,
		refused: m.settings_agentupdates_hint_refused,
		disabled: m.settings_agentupdates_hint_disabled,
		manual: m.settings_agentupdates_hint_manual,
		waiting_window: m.settings_agentupdates_hint_waiting_window,
		waiting_canary: m.settings_agentupdates_hint_waiting_canary,
		newer: m.settings_agentupdates_hint_newer,
		legacy: m.settings_agentupdates_hint_legacy,
		container: m.settings_agentupdates_hint_container,
		read_only: m.settings_agentupdates_hint_read_only,
		disabled_locally: m.settings_agentupdates_hint_disabled_locally,
		unsupported: m.settings_agentupdates_hint_unsupported,
		no_binary: m.settings_agentupdates_hint_no_binary
	};
	const OUTCOME = {
		succeeded: m.settings_agentupdates_outcome_succeeded,
		failed: m.settings_agentupdates_outcome_failed,
		rolled_back: m.settings_agentupdates_outcome_rolled_back,
		refused: m.settings_agentupdates_outcome_refused
	};
	const NO_MANUAL: AgentUpdateStatus[] = ['legacy', 'container', 'read_only', 'disabled_locally', 'unsupported', 'no_binary', 'up_to_date'];

	function enabledValue(agent: AgentUpdateAgent): string {
		return agent.enabled === null ? 'follow' : agent.enabled ? 'on' : 'off';
	}
	function setEnabled(agent: AgentUpdateAgent, value: string) {
		const enabled = value === 'follow' ? null : value === 'on';
		void act(() => setAgentUpdatePolicy(agent.id, { enabled }));
	}
</script>

<div class="border-b border-line px-5 py-4">
	<Disclosure title={m.settings_agentupdates_title()} bind:open summary={data ? (data.settings.enabled ? m.settings_agentupdates_summary_on() : m.settings_agentupdates_summary_off()) : undefined}>
		<p class="max-w-prose text-sm text-ink-2">{m.settings_agentupdates_intro()}</p>

		{#if loadError}
			<ErrorNotice error={loadError} title={m.settings_agentupdates_load_error()} class="mt-3" />
		{:else if !data}
			<Skeleton class="mt-3 h-24" />
		{:else}
			<div class="mt-4 grid gap-4">
				<div class="flex items-start gap-3">
					<Toggle id="agent-upd-enabled" checked={data.settings.enabled} disabled={busy} label={m.settings_agentupdates_enabled()} onchange={(value) => save({ enabled: value })} />
					<div class="min-w-0">
						<label for="agent-upd-enabled" class="text-sm font-semibold text-ink">{m.settings_agentupdates_enabled()}</label>
						<p class="max-w-prose text-sm text-ink-2">{m.settings_agentupdates_enabled_help()}</p>
					</div>
				</div>

				<div class="grid gap-4 sm:grid-cols-2">
					<div class="grid content-start gap-1.5">
						<label for="agent-upd-window" class="text-sm font-semibold text-ink">{m.settings_agentupdates_window()}</label>
						<select id="agent-upd-window" class="input" disabled={busy} value={data.settings.window} onchange={(e) => save({ window: e.currentTarget.value as AgentUpdateSettings['window'] })}>
							<option value="any">{m.settings_agentupdates_window_any()}</option>
							<option value="maintenance">{m.settings_agentupdates_window_maintenance()}</option>
							<option value="manual">{m.settings_agentupdates_window_manual()}</option>
						</select>
					</div>
					<div class="grid content-start gap-1.5">
						<label for="agent-upd-canary" class="text-sm font-semibold text-ink">{m.settings_agentupdates_canary_minutes()}</label>
						<input id="agent-upd-canary" type="number" inputmode="numeric" min="0" max="10080" step="1" class="input tnum" disabled={busy} bind:value={canaryInput} onchange={saveCanary} />
						<p class="max-w-prose text-sm text-ink-2">{m.settings_agentupdates_canary_help()}</p>
					</div>
				</div>

				<div class="rounded-[var(--radius-card)] border border-warning/50 bg-warning-soft px-3 py-3">
					<div class="flex items-start gap-3">
						<Toggle id="agent-upd-unsigned" checked={data.settings.allow_unsigned} disabled={busy} label={m.settings_agentupdates_unsigned()} onchange={(value) => save({ allow_unsigned: value })} />
						<div class="min-w-0">
							<label for="agent-upd-unsigned" class="text-sm font-semibold text-ink">{m.settings_agentupdates_unsigned()}</label>
							<p class="max-w-prose text-sm font-semibold text-warning-ink">{m.settings_agentupdates_unsigned_warning()}</p>
						</div>
					</div>
				</div>

				<div class="text-sm text-ink-2">
					<p class="tnum">
						{data.build ? m.settings_agentupdates_server_build({ version: data.version, build: data.build }) : m.settings_agentupdates_server_version({ version: data.version })}
					</p>
					{#if unsignedBuild}
						<p class="mt-1 max-w-prose font-semibold text-warning-ink">{m.settings_agentupdates_unsigned_build()}</p>
					{:else if data.binaries.length > 0}
						<p class="mt-1">{m.settings_agentupdates_signed()}</p>
					{/if}
				</div>

				<div class="grid gap-2 rounded-[var(--radius-card)] border border-line bg-surface-2 px-3 py-3">
					<h4 class="text-sm font-semibold text-ink">{m.settings_agentupdates_rollout_title()}</h4>
					<p class="tnum text-sm text-ink-2">
						{m.settings_agentupdates_rollout_canaries({ updated: data.rollout.canaries_updated, total: data.rollout.canaries })}
					</p>
					<p class="text-sm text-ink-2">{data.rollout.open ? m.settings_agentupdates_rollout_open() : m.settings_agentupdates_rollout_held()}</p>
					{#if data.rollout.reason}<p class="text-sm text-ink-2">{data.rollout.reason}</p>{/if}
					<div>
						<Confirm variant="secondary" size="sm" confirmLabel={m.settings_agentupdates_update_all_confirm()} loading={busy} disabled={data.agents.length === 0} onconfirm={() => act(updateAllAgentsNow)}>
							{m.settings_agentupdates_update_all()}
						</Confirm>
					</div>
				</div>

				<div>
					<h4 class="mb-2 text-sm font-semibold text-ink">{m.settings_agentupdates_agents_title()}</h4>
					{#if data.agents.length === 0}
						<p class="text-sm text-ink-2">{m.settings_agentupdates_agents_empty()}</p>
					{:else}
						<ul class="grid gap-3">
							{#each data.agents as agent (agent.id)}
								<li class="grid gap-3 rounded-[var(--radius-card)] border border-line p-3">
									<div class="flex flex-wrap items-start justify-between gap-x-3 gap-y-2">
										<div class="min-w-0">
											<p class="truncate font-semibold text-ink">{agent.name}</p>
											{#if agent.hostname && agent.hostname !== agent.name}<p class="truncate text-sm text-ink-2">{agent.hostname}</p>{/if}
											<p class="tnum text-sm text-ink-2">
												{m.settings_agentupdates_version({ version: agent.version })}{#if agent.platform} · {agent.platform}{/if}
											</p>
										</div>
										<Plate tone={TONE[agent.status]} label={LABEL[agent.status]()} />
									</div>
									{#if agent.status !== 'up_to_date'}<p class="max-w-prose text-sm text-ink-2">{HINT[agent.status]()}</p>{/if}
									{#if agent.reason}<p class="max-w-prose text-sm text-ink-2">{agent.reason}</p>{/if}
									{#if agent.last}
										<p class="text-sm text-ink-2">
											{m.settings_agentupdates_last({ outcome: OUTCOME[agent.last.outcome](), version: agent.last.version ?? '?', date: formatDateTime(agent.last.at) })}
											{#if agent.last.reason}<span class="block">{agent.last.reason}</span>{/if}
										</p>
									{/if}
									<div class="flex flex-wrap items-center gap-x-4 gap-y-2">
										<label class="flex items-center gap-2 text-sm text-ink">
											<span>{m.settings_agentupdates_policy()}</span>
											<select class="input !min-h-10 !w-auto py-0" disabled={busy} value={enabledValue(agent)} onchange={(e) => setEnabled(agent, e.currentTarget.value)}>
												<option value="follow">{m.settings_agentupdates_policy_follow()}</option>
												<option value="on">{m.settings_agentupdates_policy_on()}</option>
												<option value="off">{m.settings_agentupdates_policy_off()}</option>
											</select>
										</label>
										<label class="flex min-h-10 items-center gap-2 text-sm text-ink">
											<input type="checkbox" class="size-4" disabled={busy} checked={agent.canary} onchange={(e) => act(() => setAgentUpdatePolicy(agent.id, { canary: e.currentTarget.checked }))} />
											<span>{m.settings_agentupdates_canary_flag()}</span>
										</label>
										{#if agent.status === 'newer'}
											<Confirm variant="secondary" size="sm" confirmLabel={m.settings_agentupdates_install_server_confirm()} loading={busy} onconfirm={() => act(() => updateAgentNow(agent.id, true))}>
												{m.settings_agentupdates_install_server()}
											</Confirm>
										{:else}
											<Button variant="secondary" size="sm" disabled={busy || NO_MANUAL.includes(agent.status)} onclick={() => act(() => updateAgentNow(agent.id))}>
												{m.settings_agentupdates_update_now()}
											</Button>
										{/if}
									</div>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			</div>
		{/if}

		{#if error}<ErrorNotice {error} title={m.settings_agentupdates_save_error()} class="mt-3" />{/if}
	</Disclosure>
</div>
