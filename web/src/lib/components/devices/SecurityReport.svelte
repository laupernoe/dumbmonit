<script lang="ts">
	/**
	 * The security report of one device, opened from its badge: the grade,
	 * an unmissable "indicative, not an audit" notice, what fails (with the
	 * evidence, the fix and the vendor's guidance), the steps that raise the
	 * score on this kind of device, what passes, and — folded — the checks
	 * that could not run. Check titles, evidence and steps come from the
	 * server in English, like the collector catalog.
	 */
	import { ChevronRight, ExternalLink, Info, X } from 'lucide-svelte';
	import type { SecurityCheck, SecurityReport } from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Plate } from '#lib/ui/index.js';
	import { CATEGORY_WORD, GRADE_TONE, GRADE_WORD, SEVERITY_WORD, severityTone } from './securityFormat';

	interface Props {
		report: SecurityReport;
		id?: string;
		onclose?: () => void;
	}

	let { report, id = 'security-report', onclose }: Props = $props();

	let unknownOpen = $state(false);

	const failing = $derived(report.checks.filter((c): c is SecurityCheck => c.result === 'fail'));
	const passing = $derived(report.checks.filter((c): c is SecurityCheck => c.result === 'pass'));
	const unknown = $derived(report.checks.filter((c): c is SecurityCheck => c.result === 'unknown'));
	const linkClass = 'inline-flex min-h-10 w-fit items-center gap-1 text-[0.8125rem] font-semibold text-ink-2 hover:text-ink hover:underline';
</script>

<section {id} class="mt-6 scroll-mt-20 rounded-[var(--radius-card)] border border-line bg-surface shadow-lift rise-in" aria-labelledby={`${id}-title`}>
	<header class="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-line px-4 py-3 sm:px-5">
		<h2 id={`${id}-title`} class="text-base font-semibold tracking-tight text-ink">{m.devices_security_title()}</h2>
		{#if report.grade !== null && report.score !== null}
			<Plate tone={GRADE_TONE[report.grade]} size="md">
				{m.devices_security_grade_word({ grade: report.grade, word: GRADE_WORD[report.grade] })}
			</Plate>
			<span class="display tnum text-2xl text-ink">{report.score}<span class="text-base text-ink-3">/100</span></span>
		{:else}
			<Plate tone="ghost" size="md">{m.devices_security_not_rated()}</Plate>
		{/if}
		{#if report.capped}
			<Plate tone="warning" bare label={m.devices_security_capped()} />
		{/if}
		{#if onclose}
			<button
				type="button"
				class="ml-auto flex size-10 items-center justify-center rounded-lg text-ink-2 hover:bg-surface-2 hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal"
				aria-label={m.devices_security_close()}
				title={m.devices_security_close()}
				onclick={onclose}
			>
				<X class="size-4" aria-hidden="true" />
			</button>
		{/if}
	</header>

	<!-- Visible, not a footnote: the grade is a reading, not a verdict. -->
	<div class="flex gap-3 border-b border-line bg-info-soft px-4 py-3 sm:px-5" role="note">
		<Info class="mt-0.5 size-4 shrink-0 text-info-ink" aria-hidden="true" />
		<p class="text-sm text-ink">
			<strong class="font-semibold">{m.devices_security_disclaimer_title()}</strong>
			{m.devices_security_disclaimer()}
		</p>
	</div>

	{#if report.grade === null}
		<p class="border-b border-line px-4 py-3 text-sm text-ink-2 sm:px-5">{m.devices_security_none_evaluated()}</p>
	{/if}

	<div class="grid gap-0 lg:grid-cols-[minmax(0,1.5fr)_minmax(0,1fr)]">
		<div class="min-w-0 lg:border-r lg:border-line">
			<h3 class="px-4 pt-4 pb-1 text-[0.8125rem] font-semibold text-ink sm:px-5">{m.devices_security_missing()}</h3>
			{#if failing.length === 0}
				<p class="px-4 pb-4 text-sm text-ink-2 sm:px-5">{m.devices_security_no_failures()}</p>
			{:else}
				<ul class="divide-y divide-line">
					{#each failing as check (check.id)}
						<li class="flex flex-col gap-1.5 px-4 py-3 sm:px-5">
							<div class="flex flex-wrap items-center gap-x-3 gap-y-1.5">
								<Plate tone={severityTone(check.severity)} label={m.devices_security_failed({ severity: SEVERITY_WORD[check.severity] })} />
								<span class="min-w-0 font-semibold break-words text-ink">{check.title}</span>
								<span class="text-[0.75rem] text-ink-3">{CATEGORY_WORD[check.category]}</span>
							</div>
							<p class="text-sm break-words text-ink-2">{check.evidence}</p>
							<p class="text-sm break-words text-ink">{check.remediation}</p>
							{#if check.reference}
								<a href={check.reference} target="_blank" rel="noopener" class={linkClass}>
									{m.devices_security_guidance()}
									<ExternalLink class="size-3.5" aria-hidden="true" />
								</a>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}

			{#if passing.length > 0}
				<h3 class="border-t border-line px-4 pt-4 pb-1 text-[0.8125rem] font-semibold text-ink sm:px-5">{m.devices_security_good()}</h3>
				<ul class="divide-y divide-line">
					{#each passing as check (check.id)}
						<li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 sm:px-5">
							<Plate tone="signal" bare label={m.devices_security_passed()} />
							<span class="min-w-0 flex-1 text-sm break-words text-ink">{check.title}</span>
							<span class="min-w-0 text-[0.75rem] break-words text-ink-3">{check.evidence}</span>
						</li>
					{/each}
				</ul>
			{/if}

			{#if unknown.length > 0}
				<div class="border-t border-line">
					<button
						type="button"
						class="flex min-h-10 w-full items-center gap-2 px-4 py-2.5 text-left text-sm font-semibold text-ink-2 hover:text-ink sm:px-5"
						aria-expanded={unknownOpen}
						aria-controls={`${id}-unknown`}
						onclick={() => (unknownOpen = !unknownOpen)}
					>
						<ChevronRight class={`size-4 shrink-0 text-ink-3 transition-transform duration-200 ease-out-expo ${unknownOpen ? 'rotate-90' : ''}`} aria-hidden="true" />
						{unknown.length === 1 ? m.devices_security_unknown_one({ count: unknown.length }) : m.devices_security_unknown_other({ count: unknown.length })}
					</button>
					{#if unknownOpen}
						<ul id={`${id}-unknown`} class="divide-y divide-line border-t border-line">
							{#each unknown as check (check.id)}
								<li class="flex flex-col gap-1 px-4 py-2.5 sm:px-5">
									<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
										<Plate tone="ghost" bare label={SEVERITY_WORD[check.severity]} />
										<span class="min-w-0 text-sm font-semibold break-words text-ink">{check.title}</span>
									</div>
									<p class="text-sm break-words text-ink-2">{check.evidence}</p>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			{/if}
		</div>

		{#if report.guide}
			<div class="min-w-0 border-t border-line px-4 py-4 sm:px-5 lg:border-t-0">
				<h3 class="text-[0.8125rem] font-semibold text-ink">{m.devices_security_improve({ product: report.guide.product })}</h3>
				<ol class="mt-2 grid list-decimal gap-2 pl-5 text-sm text-ink-2">
					{#each report.guide.steps as step, i (i)}
						<li class="break-words">{step}</li>
					{/each}
				</ol>
				<a href={report.guide.doc_url} target="_blank" rel="noopener" class={`${linkClass} mt-2`}>
					{m.devices_security_docs()}
					<ExternalLink class="size-3.5" aria-hidden="true" />
				</a>
			</div>
		{/if}
	</div>

	<p class="tnum border-t border-line px-4 py-2.5 text-[0.75rem] text-ink-3 sm:px-5" title={formatDateTime(report.evaluated_at)}>
		{m.devices_security_counts({ pass: report.counts.pass, fail: report.counts.fail, unknown: report.counts.unknown })}
		· {m.devices_security_evaluated({ when: formatRelative(report.evaluated_at) })}
	</p>
</section>
