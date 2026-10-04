<script lang="ts">
	/**
	 * Security — the PingCastle / Secure Score style overview: every device
	 * whose kind has checks, worst grade first, with its top failures and a
	 * link to the full breakdown on the device page.
	 */
	import { ShieldAlert, ExternalLink } from 'lucide-svelte';
	import { getSecuritySummary } from '$lib/api/security';
	import type { SecurityGrade, SecuritySummaryDevice } from '$lib/api';
	import { formatRelative, formatDateTime } from '$lib/format';
	import { EmptyState, ErrorNotice, PageHeader, Panel, Plate, Skeleton } from '$lib/ui';
	import { GRADE_TONE, GRADE_WORD, GRADES, SUPPORTED_SECURITY_KINDS } from '$lib/components/devices/securityFormat';

	const DOCS_URL = 'https://dumbmonit.readthedocs.io/en/latest/using/security-score/';

	let devices = $state<SecuritySummaryDevice[]>([]);
	let evaluatedAt = $state<string | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const summary = await getSecuritySummary(signal);
			devices = summary.devices;
			evaluatedAt = summary.evaluated_at;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void load(controller.signal), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	/** Count per grade, plus the devices the server could not rate at all. */
	const distribution = $derived.by(() => {
		const counts: Record<SecurityGrade, number> = { A: 0, B: 0, C: 0, D: 0, F: 0 };
		let unrated = 0;
		for (const device of devices) {
			if (device.grade) counts[device.grade] += 1;
			else unrated += 1;
		}
		return { counts, unrated };
	});
</script>

<svelte:head><title>Security — DumbMonit</title></svelte:head>

<PageHeader title="Security" description="A PingCastle-style security score for every device that supports one, worst first.">
	{#snippet actions()}
		<a
			href={DOCS_URL}
			target="_blank"
			rel="noopener"
			class="inline-flex items-center gap-1 text-sm font-semibold text-ink-2 hover:text-ink hover:underline"
		>
			What is this?
			<ExternalLink class="size-3.5" aria-hidden="true" />
		</a>
	{/snippet}
</PageHeader>

{#if error}
	<ErrorNotice {error} title="Could not load the security summary" onretry={() => void load()} />
{:else if loading}
	<div class="flex flex-col gap-4" aria-busy="true" aria-label="Loading the security summary">
		<Skeleton class="h-16 w-full rounded-[var(--radius-card)]" />
		<Skeleton class="h-72 w-full rounded-[var(--radius-card)]" rows={2} />
	</div>
{:else if devices.length === 0}
	<EmptyState
		icon={ShieldAlert}
		title="No device has security checks yet."
		description={`Security checks run on: ${SUPPORTED_SECURITY_KINDS.join(', ')}, and more as they are added.`}
	/>
{:else}
	<div class="flex flex-col gap-6">
		<Panel padded={false} class="rise-in">
			<div class="flex flex-wrap items-center gap-4 px-5 py-4">
				<div class="flex flex-wrap items-center gap-2">
					{#each GRADES as grade (grade)}
						<Plate tone={GRADE_TONE[grade]} bare label={`${grade} · ${distribution.counts[grade]}`} />
					{/each}
					{#if distribution.unrated > 0}
						<Plate tone="ghost" bare label={`Not rated · ${distribution.unrated}`} />
					{/if}
				</div>
				{#if evaluatedAt}
					<span class="tnum ml-auto text-[0.75rem] text-ink-3" title={formatDateTime(evaluatedAt)}>
						evaluated {formatRelative(evaluatedAt)}
					</span>
				{/if}
			</div>
		</Panel>

		<Panel title="Devices" description="Worst score first." padded={false} class="rise-in">
			<ul class="divide-y divide-line">
				{#each devices as device, i (device.target_id)}
					<li class="rise-in flex flex-col gap-2 px-5 py-4" style="--rise-delay: {Math.min(i, 8) * 40}ms">
						<div class="flex flex-wrap items-center gap-x-3 gap-y-1.5">
							{#if device.grade !== null && device.score !== null}
								<Plate tone={GRADE_TONE[device.grade]} label={`Grade ${device.grade} · ${GRADE_WORD[device.grade]}`} />
								<span class="tnum text-sm font-semibold text-ink">{device.score}<span class="text-ink-3">/100</span></span>
							{:else}
								<Plate tone="ghost" label="Not rated" />
							{/if}
							<a href={`/targets/${device.target_id}`} class="min-w-0 truncate font-semibold text-ink hover:underline">{device.target_name}</a>
							<span class="text-[0.75rem] text-ink-3">{device.kind}</span>
							{#if device.capped}
								<Plate tone="warning" bare label="Capped" title="Capped by a failing critical check" />
							{/if}
						</div>
						<div class="tnum flex flex-wrap items-center gap-3 text-[0.8125rem] text-ink-2">
							<span>{device.pass} passed</span>
							<span>{device.fail} failed</span>
							<span>{device.unknown} not evaluated</span>
						</div>
						{#if device.top_failures.length > 0}
							<p class="text-sm text-ink-2 break-words">
								<span class="text-ink-3">Top failures:</span> {device.top_failures.join(' · ')}
							</p>
						{/if}
						<a href={`/targets/${device.target_id}`} class="text-[0.8125rem] font-semibold text-ink-2 hover:text-ink hover:underline">
							View device →
						</a>
					</li>
				{/each}
			</ul>
		</Panel>
	</div>
{/if}
