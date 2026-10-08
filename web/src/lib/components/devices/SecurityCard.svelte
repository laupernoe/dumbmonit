<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Security score of one device (PingCastle / Secure Score style): a letter
	 * grade out of a handful of checks run against the collected data — never
	 * against the device live. Shown on every device page, but only once the
	 * server confirms this kind has checks at all (`supported`); a kind without
	 * any (most of them, today) renders nothing rather than an empty card.
	 */
	import { untrack } from 'svelte';
	import { ChevronRight, ExternalLink } from 'lucide-svelte';
	import { getTargetSecurity } from '#lib/api/security.js';
	import { ApiError, type SecurityCheck, type SecurityReport, type Target } from '#lib/api/index.js';
	import { formatRelative, formatDateTime } from '#lib/format.js';
	import { ErrorNotice, Panel, Plate } from '#lib/ui/index.js';
	import { CATEGORY_WORD, GRADE_TONE, GRADE_WORD, SEVERITY_WORD, severityTone } from './securityFormat';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	const DOCS_URL = 'https://dumbmonit.readthedocs.io/en/latest/using/security-score/';

	let report = $state<SecurityReport | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);
	/** Unsupported kind, or the device vanished mid-flight: the card shows nothing. */
	let hide = $state(false);
	let unknownOpen = $state(false);
	/** Folded by default: the score is useful, rarely the first thing to read. */
	let open = $state(false);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const next = await getTargetSecurity(target.id, signal);
			report = next;
			hide = !next.supported;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			// 404 only means "this device no longer exists" on this route: the
			// device page itself already handles that case, so this card just
			// steps aside rather than piling a second error on top.
			if (cause instanceof ApiError && cause.status === 404) {
				hide = true;
				report = null;
				return;
			}
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void target.id;
		loading = true;
		report = null;
		error = null;
		hide = false;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	// --- Presentation -----------------------------------------------------------

	const failing = $derived(report?.checks.filter((c): c is SecurityCheck => c.result === 'fail') ?? []);
	const passing = $derived(report?.checks.filter((c): c is SecurityCheck => c.result === 'pass') ?? []);
	const unknown = $derived(report?.checks.filter((c): c is SecurityCheck => c.result === 'unknown') ?? []);
</script>

{#if hide}
	<!-- Nothing to show: either the kind has no security checks, or the device no longer exists. -->
{:else if error}
	<section class="mt-6" aria-label={m.devices_security_title()}>
		<Panel title={m.devices_security_title()} class="rise-in">
			<ErrorNotice {error} title={m.devices_security_error()} onretry={() => void load()} />
		</Panel>
	</section>
{:else if loading || !report}
	<!-- Nothing while the first answer is pending: most kinds have no checks,
	     and a skeleton that then vanishes would make the page jump. -->
{:else}
	<section class="mt-6" aria-label={m.devices_security_title()}>
	<Panel
		title={m.devices_security_title()}
		description={m.devices_security_counts({ pass: report.counts.pass, fail: report.counts.fail, unknown: report.counts.unknown })}
		padded={false}
		class="rise-in"
	>
		{#snippet aside()}
			{#if report && report.grade !== null && report.score !== null}
				<Plate tone={GRADE_TONE[report.grade]} label={m.devices_security_grade_score({ grade: report.grade, score: report.score })} />
			{/if}
			<button
				type="button"
				class="inline-flex min-h-10 items-center gap-1 text-[0.8125rem] font-semibold text-ink-2 hover:text-ink"
				aria-expanded={open}
				onclick={() => (open = !open)}
			>
				<ChevronRight class={`size-4 transition-transform duration-200 ease-out-expo ${open ? 'rotate-90' : ''}`} aria-hidden="true" />
				{open ? m.devices_security_hide() : m.devices_security_details()}
			</button>
			<a
				href={DOCS_URL}
				target="_blank"
				rel="noopener"
				class="inline-flex min-h-10 items-center gap-1 text-[0.8125rem] font-semibold text-ink-2 hover:text-ink hover:underline"
			>
				{m.devices_security_what()}
				<ExternalLink class="size-3.5" aria-hidden="true" />
			</a>
		{/snippet}

		{#if open}
		<div class="flex flex-wrap items-center gap-4 border-b border-line px-5 py-4">
			{#if report.grade !== null && report.score !== null}
				<div class="flex items-center gap-3">
					<Plate tone={GRADE_TONE[report.grade]} size="md">
						{m.devices_security_grade_word({ grade: report.grade, word: GRADE_WORD[report.grade] })}
					</Plate>
					<span class="display tnum text-3xl text-ink">{report.score}<span class="text-lg text-ink-3">/100</span></span>
				</div>
			{:else}
				<div class="flex items-center gap-3">
					<Plate tone="ghost" size="md">{m.devices_security_not_rated()}</Plate>
					<span class="text-sm text-ink-2">{m.devices_security_none_evaluated()}</span>
				</div>
			{/if}
			{#if report.capped}
				<Plate tone="warning" bare label={m.devices_security_capped()} />
			{/if}
			<span class="tnum ml-auto text-[0.75rem] text-ink-3" title={formatDateTime(report.evaluated_at)}>
				{m.devices_security_evaluated({ when: formatRelative(report.evaluated_at) })}
			</span>
		</div>

		{#if report.checks.length === 0}
			<p class="px-5 py-4 text-sm text-ink-2">{m.devices_security_no_checks()}</p>
		{:else}
			<div class="flex flex-col">
				{#if failing.length > 0}
					<ul class="divide-y divide-line">
						{#each failing as check, i (check.id)}
							<li class="rise-in flex flex-col gap-1.5 px-5 py-3" style="--rise-delay: {Math.min(i, 8) * 40}ms">
								<div class="flex flex-wrap items-center gap-x-3 gap-y-1.5">
									<Plate tone={severityTone(check.severity)} label={m.devices_security_failed({ severity: SEVERITY_WORD[check.severity] })} />
									<span class="min-w-0 font-semibold text-ink break-words">{check.title}</span>
									<span class="text-[0.75rem] text-ink-3">{CATEGORY_WORD[check.category]}</span>
								</div>
								<p class="text-sm text-ink-2 break-words">{check.evidence}</p>
								<p class="text-sm text-ink break-words">{check.remediation}</p>
								{#if check.reference}
									<a
										href={check.reference}
										target="_blank"
										rel="noopener"
										class="inline-flex w-fit items-center gap-1 text-[0.8125rem] font-semibold text-ink-2 hover:text-ink hover:underline"
									>
										{m.devices_security_guidance()}
										<ExternalLink class="size-3.5" aria-hidden="true" />
									</a>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}

				{#if passing.length > 0}
					<ul class="divide-y divide-line">
						{#each passing as check (check.id)}
							<li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-5 py-2.5">
								<Plate tone="signal" bare label={m.devices_security_passed()} />
								<span class="min-w-0 flex-1 text-sm text-ink break-words">{check.title}</span>
								<span class="text-[0.75rem] text-ink-3">{CATEGORY_WORD[check.category]}</span>
							</li>
						{/each}
					</ul>
				{/if}

				{#if unknown.length > 0}
					<div class="border-t border-line">
						<button
							type="button"
							class="flex min-h-10 w-full items-center gap-2 px-4 py-2.5 text-left sm:px-5 text-sm font-semibold text-ink-2 hover:text-ink"
							aria-expanded={unknownOpen}
							aria-controls="security-unknown-checks"
							onclick={() => (unknownOpen = !unknownOpen)}
						>
							<ChevronRight class={`size-4 shrink-0 text-ink-3 transition-transform duration-200 ease-out-expo ${unknownOpen ? 'rotate-90' : ''}`} aria-hidden="true" />
							{unknown.length === 1 ? m.devices_security_unknown_one({ count: unknown.length }) : m.devices_security_unknown_other({ count: unknown.length })}
						</button>
						{#if unknownOpen}
							<ul id="security-unknown-checks" class="divide-y divide-line border-t border-line">
								{#each unknown as check (check.id)}
									<li class="flex flex-col gap-1 px-5 py-2.5">
										<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
											<Plate tone="ghost" bare label={SEVERITY_WORD[check.severity]} />
											<span class="min-w-0 text-sm font-semibold text-ink break-words">{check.title}</span>
											<span class="text-[0.75rem] text-ink-3">{CATEGORY_WORD[check.category]}</span>
										</div>
										<p class="text-sm text-ink-2 break-words">{check.evidence}</p>
									</li>
								{/each}
							</ul>
						{/if}
					</div>
				{/if}
			</div>
		{/if}
		{/if}
	</Panel>
	</section>
{/if}
