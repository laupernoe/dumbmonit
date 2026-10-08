<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * What an Active Directory domain has to show beyond charts, in the order
	 * an administrator should look: the findings first (who can be roasted,
	 * who delegates without constraint, how old the krbtgt password is), then
	 * the domain controllers and whether each one answers, then who sits in
	 * the privileged groups, then the domain itself, its policy and counts.
	 * One read of what the probe stored, refreshed every minute; the domain
	 * controller is never asked because a page was opened.
	 */
	import { untrack } from 'svelte';
	import { getAdOverview } from '#lib/api/activedirectory.js';
	import type { AdFindingSeverity, AdOverview, Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import type { Tone } from '#lib/ui/index.js';
	import { formatAgo, formatCount, formatSpan, formatUnix } from '../truenas/format';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let overview = $state<AdOverview | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);
	/** Findings whose sample list is unfolded. */
	let open = $state<Record<string, boolean>>({});

	/** The score's scale (low → critical), in the interface's words. */
	const SEVERITY: Record<AdFindingSeverity, { tone: Tone; label: string }> = {
		get critical() {
			return { tone: 'warning' as Tone, label: m.devices_ad_sev_warning() };
		},
		get high() {
			return { tone: 'warning' as Tone, label: m.devices_ad_sev_warning() };
		},
		get medium() {
			return { tone: 'advisory' as Tone, label: m.devices_ad_sev_advisory() };
		},
		get low() {
			return { tone: 'info' as Tone, label: m.devices_ad_sev_info() };
		}
	};

	const ROLE_LABEL: Record<string, string> = {
		get schema() {
			return m.devices_ad_role_schema();
		},
		get domain_naming() {
			return m.devices_ad_role_naming();
		},
		pdc: 'PDC',
		rid: 'RID',
		get infrastructure() {
			return m.devices_ad_role_infra();
		}
	};

	/** "1 object" / "3 objects": the singular and plural messages, with a grouped count. */
	function pl(one: (p: { count: string }) => string, other: (p: { count: string }) => string, n: number): string {
		return (n === 1 ? one : other)({ count: formatCount(n) });
	}

	const domain = $derived(overview?.domain ?? null);
	const inventory = $derived(overview?.inventory ?? null);
	const policy = $derived(overview?.policy ?? null);
	/** Findings examined with nothing found (count 0) stay out of the list. */
	const findings = $derived((overview?.findings ?? []).filter((finding) => finding.count > 0));
	const warnings = $derived(
		(overview?.finding_counts.critical ?? 0) + (overview?.finding_counts.high ?? 0)
	);
	const advisories = $derived(overview?.finding_counts.medium ?? 0);
	const unreachable = $derived((overview?.dcs ?? []).filter((dc) => dc.reachability === 'unreachable'));
	const replication = $derived(overview?.replication ?? null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			overview = await getAdOverview(target.id, signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void target.id;
		loading = true;
		overview = null;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	function days(seconds: number | null): string {
		if (seconds === null) return m.devices_ad_never();
		return pl(m.devices_ad_days_one, m.devices_ad_days_other, Math.round(seconds / 86400));
	}

	function reachPlate(reachability: string): { tone: Tone; label: string } {
		switch (reachability) {
			case 'reachable':
				return { tone: 'signal', label: m.devices_ad_reach_ok() };
			case 'unreachable':
				return { tone: 'warning', label: m.devices_ad_reach_down() };
			case 'unresolved':
				return { tone: 'ghost', label: m.devices_ad_reach_unresolved() };
			default:
				return { tone: 'ghost', label: m.devices_ad_reach_unchecked() };
		}
	}
</script>

{#if error}
	<Panel title="Active Directory" class="rise-in">
		<ErrorNotice {error} title={m.devices_ad_error()} onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title="Active Directory" padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-5 py-4" aria-busy="true" aria-label={m.devices_ad_loading()}>
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else if !overview || overview.probed_at === null}
	<Panel title="Active Directory" class="rise-in">
		<p class="text-sm text-ink-2">{m.devices_ad_unread()}</p>
	</Panel>
{:else if overview.bind_error}
	<Panel title="Active Directory" class="rise-in">
		{#snippet aside()}
			<Plate tone="warning" label={m.devices_ad_bind_refused()} />
		{/snippet}
		<div class="flex flex-col gap-2">
			<p class="text-sm text-ink">
				{m.devices_ad_bind_text({ host: overview.connection?.host ?? m.devices_ad_dc_default(), error: overview.bind_error })}
			</p>
			<p class="text-[0.8125rem] text-ink-3">
				{m.devices_ad_bind_hint({ when: formatAgo(overview.probed_at) })}
			</p>
		</div>
	</Panel>
{:else}
	<div class="flex flex-col gap-6">
		<Panel
			title={m.devices_ad_findings_title()}
			description={m.devices_ad_findings_desc()}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if warnings > 0}
					<Plate tone="warning" label={pl(m.devices_ad_warnings_one, m.devices_ad_warnings_other, warnings)} />
				{:else if advisories > 0}
					<Plate tone="advisory" label={pl(m.devices_ad_advisories_one, m.devices_ad_advisories_other, advisories)} />
				{:else if findings.length === 0 && inventory}
					<Plate tone="signal" label={m.devices_ad_nothing_found()} />
				{/if}
			{/snippet}
			{#if findings.length === 0}
				<p class="px-5 py-4 text-sm text-ink-2">
					{inventory
						? m.devices_ad_no_finding()
						: m.devices_ad_inventory_pending()}
				</p>
			{:else}
				<ul class="flex flex-col divide-y divide-line">
					{#each findings as finding (finding.id)}
						{@const plate = SEVERITY[finding.severity] ?? { tone: 'ghost', label: finding.severity }}
						<li class="flex flex-col gap-1.5 px-5 py-3">
							<div class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-3">
								<span class="shrink-0"><Plate tone={plate.tone} label={plate.label} /></span>
								<span class="min-w-0 flex-1 text-sm font-semibold break-words text-ink">{finding.title}</span>
								{#if finding.samples.length > 0}
									<span class="tnum shrink-0 text-[0.8125rem] text-ink-2">
										{pl(m.devices_ad_objects_one, m.devices_ad_objects_other, finding.count)}
									</span>
								{/if}
							</div>
							<p class="text-[0.8125rem] text-ink-2">{finding.detail}</p>
							{#if finding.samples.length > 0}
								{@const shown = open[finding.id] ? finding.samples : finding.samples.slice(0, 5)}
								<p class="text-[0.75rem] break-words text-ink-3">
									<span class="font-mono">{shown.join(', ')}</span>
									{#if finding.samples.length > 5}
										<button
											type="button"
											class="ml-1 inline-flex min-h-10 items-center text-ink-2 underline underline-offset-2 hover:text-ink sm:min-h-0"
											onclick={() => (open[finding.id] = !open[finding.id])}
										>
											{open[finding.id] ? m.devices_ad_fewer() : m.devices_ad_more({ count: finding.samples.length - 5 })}
										</button>
									{/if}
									{#if finding.count > finding.samples.length}
										<span>{m.devices_ad_not_listed({ count: formatCount(finding.count - finding.samples.length) })}</span>
									{/if}
								</p>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</Panel>

		<Panel
			title={m.devices_ad_dcs_title()}
			description={m.devices_ad_dcs_desc()}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if unreachable.length > 0}
					<Plate tone="warning" label={m.devices_ad_unreachable_count({ count: formatCount(unreachable.length) })} />
				{:else if replication && replication.failing > 0}
					<Plate tone="warning" label={m.devices_ad_replication_failing()} />
				{/if}
			{/snippet}
			<ul class="flex flex-col divide-y divide-line">
				{#each overview.dcs as dc (dc.name)}
					{@const reach = reachPlate(dc.reachability)}
					<li class="flex flex-col gap-1 px-5 py-3 sm:flex-row sm:items-center sm:gap-3">
						<span class="shrink-0" title={dc.reachability_detail ?? undefined}>
							<Plate tone={reach.tone} label={reach.label} />
						</span>
						<div class="min-w-0 flex-1">
							<p class="text-sm break-words text-ink">
								<span class="font-semibold">{dc.host_name ?? dc.name}</span>
								{#if dc.queried}<span class="text-ink-3"> {m.devices_ad_queried()}</span>{/if}
							</p>
							<p class="tnum flex flex-wrap gap-x-3 text-[0.75rem] text-ink-3">
								<span>{m.devices_ad_site({ site: dc.site })}</span>
								{#if dc.global_catalog}<span>{m.devices_ad_gc()}</span>{/if}
								{#if dc.read_only}<span>{m.devices_ad_readonly()}</span>{/if}
								{#if dc.operating_system}<span>{dc.operating_system}</span>{/if}
								{#if dc.roles.length > 0}
									<span>{m.devices_ad_roles({ roles: dc.roles.map((role) => ROLE_LABEL[role] ?? role).join(', ') })}</span>
								{/if}
							</p>
							{#if dc.reachability_detail && dc.reachability !== 'reachable'}
								<p class="text-[0.75rem] break-words text-ink-3">{dc.reachability_detail}</p>
							{/if}
						</div>
					</li>
				{:else}
					<li class="px-5 py-3 text-sm text-ink-2">{m.devices_ad_no_dc()}</li>
				{/each}
			</ul>
			{#if replication}
				<div class="flex flex-col gap-1 border-t border-line px-5 py-3">
					<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devices_ad_repl_title()}</p>
					{#if replication.status === 'single_dc'}
						<p class="text-sm text-ink-2">{m.devices_ad_repl_single()}</p>
					{:else if replication.status === 'not_readable'}
						<p class="text-sm text-ink-2">
							{m.devices_ad_repl_unreadable()}
						</p>
					{:else}
						<ul class="flex flex-col gap-1">
							{#each replication.neighbors as neighbor, index (`${neighbor.source}/${neighbor.naming_context}/${index}`)}
								<li class="flex flex-wrap items-center gap-2 text-[0.8125rem]">
									{#if neighbor.consecutive_failures > 0}
										<Plate tone="warning" label={m.devices_ad_repl_failed({ count: neighbor.consecutive_failures })} />
									{:else}
										<Plate tone="signal" label={m.devices_ad_repl_sync()} />
									{/if}
									<span class="font-semibold text-ink">{neighbor.source}</span>
									<span class="font-mono text-[0.75rem] break-all text-ink-3">{neighbor.naming_context}</span>
									<span class="tnum text-ink-3" title={formatUnix(neighbor.last_success)}>
										{neighbor.last_result !== 0 ? m.devices_ad_repl_last_err({ ago: formatAgo(neighbor.last_success), code: neighbor.last_result }) : m.devices_ad_repl_last({ ago: formatAgo(neighbor.last_success) })}
									</span>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			{/if}
		</Panel>

		<Panel
			title={m.devices_ad_groups_title()}
			description={m.devices_ad_groups_desc()}
			padded={false}
			class="rise-in"
		>
			<ul class="flex flex-col divide-y divide-line">
				{#each overview.privileged_groups as group (group.rid)}
					<li class="flex flex-col gap-1 px-5 py-3">
						<div class="flex flex-wrap items-baseline gap-x-3 gap-y-0.5">
							<span class="text-sm font-semibold text-ink">{group.name}</span>
							{#if group.found}
								<span class="tnum text-[0.8125rem] text-ink-2">
									{[
										pl(m.devices_ad_members_one, m.devices_ad_members_other, group.member_count),
										group.enabled_member_count !== group.member_count
											? m.devices_ad_members_enabled({ count: formatCount(group.enabled_member_count) })
											: '',
										group.nested_group_count > 0
											? pl(m.devices_ad_nested_one, m.devices_ad_nested_other, group.nested_group_count)
											: ''
									]
										.filter(Boolean)
										.join(', ')}
								</span>
							{:else}
								<span class="text-[0.8125rem] text-ink-3">{m.devices_ad_forest_root()}</span>
							{/if}
						</div>
						{#if group.members.length > 0}
							<p class="flex flex-wrap gap-x-2 gap-y-0.5 text-[0.75rem]">
								{#each group.members as member (member.name)}
									<span class={member.enabled ? 'font-mono text-ink-2' : 'font-mono text-ink-3 line-through'} title={member.enabled ? member.kind : m.devices_ad_member_disabled({ kind: member.kind })}>
										{member.name}{#if !member.enabled}<span class="sr-only"> {m.devices_ad_disabled_sr()}</span>{/if}
									</span>
								{/each}
								{#if group.truncated}<span class="text-ink-3">…</span>{/if}
							</p>
						{/if}
					</li>
				{/each}
			</ul>
		</Panel>

		<Panel
			title={m.devices_ad_domain_title()}
			description={domain ? m.devices_ad_domain_desc({ dns: domain.dns_name, forest: domain.forest_dns_name }) : undefined}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				<span class="tnum text-[0.75rem] text-ink-3" title={formatUnix(overview?.probed_at)}>
					{m.devices_ad_read_ago({ ago: formatAgo(overview?.probed_at) })}
				</span>
			{/snippet}
			<div class="flex flex-col divide-y divide-line">
				{#if domain}
					<div class="flex flex-col gap-1 px-5 py-3">
						<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devices_ad_domain_title()}</p>
						<p class="tnum flex flex-wrap gap-x-4 gap-y-0.5 text-[0.8125rem] text-ink-2">
							{#if domain.domain_level_label}<span>{m.devices_ad_domain_level({ level: domain.domain_level_label })}</span>{/if}
							{#if domain.forest_level_label}<span>{m.devices_ad_forest_level({ level: domain.forest_level_label })}</span>{/if}
							{#if domain.dc_host_name}<span>{m.devices_ad_via({ host: domain.dc_host_name })}</span>{/if}
							{#if overview.connection}
								<span>
									{overview.connection.certificate_verified
										? overview.connection.security.toUpperCase()
										: m.devices_ad_cert_unverified({ security: overview.connection.security.toUpperCase() })}
								</span>
							{/if}
							{#if domain.clock_skew_seconds !== null}
								<span>{Math.abs(domain.clock_skew_seconds) < 2 ? m.devices_ad_clock_sync() : m.devices_ad_clock_skew({ offset: `${domain.clock_skew_seconds > 0 ? '+' : '−'}${formatSpan(Math.abs(domain.clock_skew_seconds))}` })}</span>
							{/if}
							{#if domain.recycle_bin_enabled !== null}
								<span>{domain.recycle_bin_enabled ? m.devices_ad_recycle_on() : m.devices_ad_recycle_off()}</span>
							{/if}
						</p>
					</div>
				{/if}
				{#if inventory}
					<div class="flex flex-col gap-1 px-5 py-3">
						<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devices_ad_accounts()}</p>
						<p class="tnum flex flex-wrap gap-x-4 gap-y-0.5 text-[0.8125rem] text-ink-2">
							<span>{m.devices_ad_users_enabled({ enabled: formatCount(inventory.users_enabled), total: formatCount(inventory.users_total) })}</span>
							<span>{m.devices_ad_locked_out({ count: formatCount(inventory.users_locked_out) })}</span>
							<span>{m.devices_ad_inactive({ count: formatCount(inventory.users_stale), days: inventory.stale_days_users })}</span>
							<span>{m.devices_ad_computers_enabled({ enabled: formatCount(inventory.computers_enabled), total: formatCount(inventory.computers_total) })}</span>
							<span>{m.devices_ad_groups_total({ count: formatCount(inventory.groups_total) })}</span>
							<span>{m.devices_ad_laps({ enabled: formatCount(inventory.laps_computers), eligible: formatCount(inventory.laps_eligible_computers) })}</span>
							<span>{inventory.krbtgt_password_age_seconds === null ? m.devices_ad_krbtgt_unknown() : m.devices_ad_krbtgt_age({ age: days(inventory.krbtgt_password_age_seconds) })}</span>
						</p>
						<p class="text-[0.75rem] text-ink-3" title={formatUnix(inventory.refreshed_at)}>
							{m.devices_ad_inventory_read({ ago: formatAgo(inventory.refreshed_at) })}
						</p>
					</div>
				{:else}
					<p class="px-5 py-3 text-[0.8125rem] text-ink-3">{m.devices_ad_inventory_loading()}</p>
				{/if}
				{#if policy}
					<div class="flex flex-col gap-1 px-5 py-3">
						<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devices_ad_policy_title()}</p>
						<p class="tnum flex flex-wrap gap-x-4 gap-y-0.5 text-[0.8125rem] text-ink-2">
							{#if policy.min_password_length !== null}<span>{m.devices_ad_policy_min({ count: policy.min_password_length })}</span>{/if}
							{#if policy.complexity_required !== null}<span>{policy.complexity_required ? m.devices_ad_policy_complex_yes() : m.devices_ad_policy_complex_no()}</span>{/if}
							<span>{m.devices_ad_policy_max_age({ age: days(policy.max_password_age_seconds) })}</span>
							{#if policy.lockout_threshold !== null}
								<span>{policy.lockout_threshold === 0 ? m.devices_ad_policy_no_lockout() : m.devices_ad_policy_lockout({ count: policy.lockout_threshold })}</span>
							{/if}
							{#if policy.machine_account_quota !== null}<span>{m.devices_ad_policy_quota({ count: policy.machine_account_quota })}</span>{/if}
						</p>
					</div>
				{/if}
				{#if overview.errors.length > 0}
					<div class="flex flex-col gap-1 px-5 py-3">
						<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devices_ad_not_read()}</p>
						<ul class="flex flex-col gap-0.5 text-[0.75rem] break-words text-ink-3">
							{#each overview.errors as message, index (index)}
								<li>{message}</li>
							{/each}
						</ul>
					</div>
				{/if}
			</div>
		</Panel>
	</div>
{/if}
