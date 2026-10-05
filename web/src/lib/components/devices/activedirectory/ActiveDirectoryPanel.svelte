<script lang="ts">
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
	import { getAdOverview } from '$lib/api/activedirectory';
	import type { AdFindingSeverity, AdOverview, Target } from '$lib/api';
	import { ErrorNotice, Panel, Plate, Skeleton } from '$lib/ui';
	import type { Tone } from '$lib/ui';
	import { formatAgo, formatCount, formatSpan, formatUnix, plural } from '../truenas/format';

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
		critical: { tone: 'warning', label: 'Warning' },
		high: { tone: 'warning', label: 'Warning' },
		medium: { tone: 'advisory', label: 'Advisory' },
		low: { tone: 'info', label: 'Info' }
	};

	const ROLE_LABEL: Record<string, string> = {
		schema: 'Schema',
		domain_naming: 'Naming',
		pdc: 'PDC',
		rid: 'RID',
		infrastructure: 'Infra'
	};

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
		if (seconds === null) return 'never';
		return `${formatCount(Math.round(seconds / 86400))} days`;
	}

	function reachPlate(reachability: string): { tone: Tone; label: string } {
		switch (reachability) {
			case 'reachable':
				return { tone: 'signal', label: 'Answers' };
			case 'unreachable':
				return { tone: 'warning', label: 'Unreachable' };
			case 'unresolved':
				return { tone: 'ghost', label: 'Name unresolved' };
			default:
				return { tone: 'ghost', label: 'Not checked' };
		}
	}
</script>

{#if error}
	<Panel title="Active Directory" class="rise-in">
		<ErrorNotice {error} title="Could not load the domain" onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title="Active Directory" padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-5 py-4" aria-busy="true" aria-label="Loading the domain">
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else if !overview || overview.probed_at === null}
	<Panel title="Active Directory" class="rise-in">
		<p class="text-sm text-ink-2">The domain has not been read yet.</p>
	</Panel>
{:else if overview.bind_error}
	<Panel title="Active Directory" class="rise-in">
		{#snippet aside()}
			<Plate tone="warning" label="Bind refused" />
		{/snippet}
		<div class="flex flex-col gap-2">
			<p class="text-sm text-ink">
				{overview.connection?.host ?? 'The domain controller'} answers but refuses the service account:
				<span class="font-semibold">{overview.bind_error}</span>.
			</p>
			<p class="text-[0.8125rem] text-ink-3">
				Nothing else is read until the account can log in again. Read {formatAgo(overview.probed_at)}.
			</p>
		</div>
	</Panel>
{:else}
	<div class="flex flex-col gap-6">
		<Panel
			title="Security findings"
			description="Raw facts read from the directory, most severe first. Account findings come from the full inventory, read in the background."
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if warnings > 0}
					<Plate tone="warning" label={plural(warnings, 'warning')} />
				{:else if advisories > 0}
					<Plate tone="advisory" label={plural(advisories, 'advisory', 'advisories')} />
				{:else if findings.length === 0 && inventory}
					<Plate tone="signal" label="Nothing found" />
				{/if}
			{/snippet}
			{#if findings.length === 0}
				<p class="px-5 py-4 text-sm text-ink-2">
					{inventory
						? 'No finding: nothing in the directory matches a known weakness.'
						: 'The full inventory has not completed yet; policy and infrastructure checks found nothing.'}
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
										{plural(finding.count, 'object')}
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
											class="ml-1 text-ink-2 underline underline-offset-2 hover:text-ink"
											onclick={() => (open[finding.id] = !open[finding.id])}
										>
											{open[finding.id] ? 'fewer' : `${finding.samples.length - 5} more`}
										</button>
									{/if}
									{#if finding.count > finding.samples.length}
										<span>· {formatCount(finding.count - finding.samples.length)} not listed</span>
									{/if}
								</p>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</Panel>

		<Panel
			title="Domain controllers"
			description="Every domain controller of the domain, from the configuration partition, and whether its LDAP port answers."
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if unreachable.length > 0}
					<Plate tone="warning" label={`${formatCount(unreachable.length)} unreachable`} />
				{:else if replication && replication.failing > 0}
					<Plate tone="warning" label="Replication failing" />
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
								{#if dc.queried}<span class="text-ink-3"> · queried</span>{/if}
							</p>
							<p class="tnum flex flex-wrap gap-x-3 text-[0.75rem] text-ink-3">
								<span>site {dc.site}</span>
								{#if dc.global_catalog}<span>global catalog</span>{/if}
								{#if dc.read_only}<span>read-only</span>{/if}
								{#if dc.operating_system}<span>{dc.operating_system}</span>{/if}
								{#if dc.roles.length > 0}
									<span>roles {dc.roles.map((role) => ROLE_LABEL[role] ?? role).join(', ')}</span>
								{/if}
							</p>
							{#if dc.reachability_detail && dc.reachability !== 'reachable'}
								<p class="text-[0.75rem] break-words text-ink-3">{dc.reachability_detail}</p>
							{/if}
						</div>
					</li>
				{:else}
					<li class="px-5 py-3 text-sm text-ink-2">No domain controller could be read from the configuration partition.</li>
				{/each}
			</ul>
			{#if replication}
				<div class="flex flex-col gap-1 border-t border-line px-5 py-3">
					<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">Inbound replication</p>
					{#if replication.status === 'single_dc'}
						<p class="text-sm text-ink-2">A single domain controller: nothing to replicate.</p>
					{:else if replication.status === 'not_readable'}
						<p class="text-sm text-ink-2">
							Not readable: the service account lacks the Monitor active directory replication right.
						</p>
					{:else}
						<ul class="flex flex-col gap-1">
							{#each replication.neighbors as neighbor, index (`${neighbor.source}/${neighbor.naming_context}/${index}`)}
								<li class="flex flex-wrap items-center gap-2 text-[0.8125rem]">
									{#if neighbor.consecutive_failures > 0}
										<Plate tone="warning" label={`${neighbor.consecutive_failures} failed`} />
									{:else}
										<Plate tone="signal" label="In sync" />
									{/if}
									<span class="font-semibold text-ink">{neighbor.source}</span>
									<span class="font-mono text-[0.75rem] break-all text-ink-3">{neighbor.naming_context}</span>
									<span class="tnum text-ink-3" title={formatUnix(neighbor.last_success)}>
										last success {formatAgo(neighbor.last_success)}{#if neighbor.last_result !== 0}, error {neighbor.last_result}{/if}
									</span>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			{/if}
		</Panel>

		<Panel
			title="Privileged groups"
			description="Effective members, nested groups included, found by their well-known identifier whatever the domain's language."
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
									{plural(group.member_count, 'member')}{#if group.enabled_member_count !== group.member_count}, {formatCount(group.enabled_member_count)} enabled{/if}{#if group.nested_group_count > 0}, through {plural(group.nested_group_count, 'nested group')}{/if}
								</span>
							{:else}
								<span class="text-[0.8125rem] text-ink-3">in the forest root domain</span>
							{/if}
						</div>
						{#if group.members.length > 0}
							<p class="flex flex-wrap gap-x-2 gap-y-0.5 text-[0.75rem]">
								{#each group.members as member (member.name)}
									<span class={member.enabled ? 'font-mono text-ink-2' : 'font-mono text-ink-3 line-through'} title={member.enabled ? member.kind : `${member.kind}, disabled`}>
										{member.name}{#if !member.enabled}<span class="sr-only"> (disabled)</span>{/if}
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
			title="Domain"
			description={domain ? `${domain.dns_name}, forest ${domain.forest_dns_name}.` : undefined}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				<span class="tnum text-[0.75rem] text-ink-3" title={formatUnix(overview?.probed_at)}>
					read {formatAgo(overview?.probed_at)}
				</span>
			{/snippet}
			<div class="flex flex-col divide-y divide-line">
				{#if domain}
					<div class="flex flex-col gap-1 px-5 py-3">
						<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">Domain</p>
						<p class="tnum flex flex-wrap gap-x-4 gap-y-0.5 text-[0.8125rem] text-ink-2">
							{#if domain.domain_level_label}<span>domain level {domain.domain_level_label}</span>{/if}
							{#if domain.forest_level_label}<span>forest level {domain.forest_level_label}</span>{/if}
							{#if domain.dc_host_name}<span>via {domain.dc_host_name}</span>{/if}
							{#if overview.connection}
								<span>
									{overview.connection.security.toUpperCase()}{#if !overview.connection.certificate_verified}, certificate not verified{/if}
								</span>
							{/if}
							{#if domain.clock_skew_seconds !== null}
								<span>clock {Math.abs(domain.clock_skew_seconds) < 2 ? 'in step' : `${domain.clock_skew_seconds > 0 ? '+' : '−'}${formatSpan(Math.abs(domain.clock_skew_seconds))}`}</span>
							{/if}
							{#if domain.recycle_bin_enabled !== null}
								<span>Recycle Bin {domain.recycle_bin_enabled ? 'on' : 'off'}</span>
							{/if}
						</p>
					</div>
				{/if}
				{#if inventory}
					<div class="flex flex-col gap-1 px-5 py-3">
						<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">Accounts</p>
						<p class="tnum flex flex-wrap gap-x-4 gap-y-0.5 text-[0.8125rem] text-ink-2">
							<span>{formatCount(inventory.users_enabled)} of {formatCount(inventory.users_total)} users enabled</span>
							<span>{formatCount(inventory.users_locked_out)} locked out</span>
							<span>{formatCount(inventory.users_stale)} inactive over {inventory.stale_days_users} days</span>
							<span>{formatCount(inventory.computers_enabled)} of {formatCount(inventory.computers_total)} computers enabled</span>
							<span>{formatCount(inventory.groups_total)} groups</span>
							<span>LAPS on {formatCount(inventory.laps_computers)} of {formatCount(inventory.laps_eligible_computers)} computers</span>
							<span>krbtgt password {inventory.krbtgt_password_age_seconds === null ? 'age unknown' : `${days(inventory.krbtgt_password_age_seconds)} old`}</span>
						</p>
						<p class="text-[0.75rem] text-ink-3" title={formatUnix(inventory.refreshed_at)}>
							Full inventory read {formatAgo(inventory.refreshed_at)}.
						</p>
					</div>
				{:else}
					<p class="px-5 py-3 text-[0.8125rem] text-ink-3">The full inventory of accounts is being read in the background.</p>
				{/if}
				{#if policy}
					<div class="flex flex-col gap-1 px-5 py-3">
						<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">Default password policy</p>
						<p class="tnum flex flex-wrap gap-x-4 gap-y-0.5 text-[0.8125rem] text-ink-2">
							{#if policy.min_password_length !== null}<span>minimum {policy.min_password_length} characters</span>{/if}
							{#if policy.complexity_required !== null}<span>complexity {policy.complexity_required ? 'required' : 'not required'}</span>{/if}
							<span>maximum age {policy.max_password_age_seconds === null ? 'never' : days(policy.max_password_age_seconds)}</span>
							{#if policy.lockout_threshold !== null}
								<span>{policy.lockout_threshold === 0 ? 'no lockout' : `lockout after ${policy.lockout_threshold} failures`}</span>
							{/if}
							{#if policy.machine_account_quota !== null}<span>machine account quota {policy.machine_account_quota}</span>{/if}
						</p>
					</div>
				{/if}
				{#if overview.errors.length > 0}
					<div class="flex flex-col gap-1 px-5 py-3">
						<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">Not read</p>
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
