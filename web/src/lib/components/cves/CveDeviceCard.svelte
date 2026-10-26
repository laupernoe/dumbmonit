<script lang="ts">
	/**
	 * Device page: this device's known vulnerabilities, compact. Renders nothing
	 * while the feature is off, when the device has no inventoried component,
	 * or when a read fails (the Security page is the place for errors).
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { getCveSummary, listCves } from '#lib/api/cves.js';
	import type { CveFinding, CveSummary, TargetId } from '#lib/api/index.js';
	import { Plate } from '#lib/ui/index.js';
	import { severityTone, stateTone, topFindings } from '#lib/cves.js';

	interface Props {
		targetId: TargetId;
	}
	let { targetId }: Props = $props();

	let summary = $state<CveSummary | null>(null);
	let findings = $state<CveFinding[]>([]);

	$effect(() => {
		const id = targetId;
		const controller = new AbortController();
		summary = null;
		findings = [];
		(async () => {
			try {
				const s = await getCveSummary(id, controller.signal);
				if (!s.enabled || s.components === 0) return;
				const list = await listCves({ target: id, ignored: false }, controller.signal);
				summary = s;
				findings = list.findings;
			} catch {
				/* quiet: nothing to show */
			}
		})();
		return () => controller.abort();
	});

	const top = $derived(topFindings(findings, 5));
	const STATE = {
		vulnerable: () => m.security_cves_state_vulnerable(),
		unknown: () => m.security_cves_state_unknown(),
		patched: () => m.security_cves_state_patched()
	};
</script>

{#if summary}
	<section class="mt-6 rounded-[var(--radius-card)] border border-line bg-surface px-4 py-3" aria-labelledby="device-cve">
		<div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
			<h2 id="device-cve" class="text-sm font-semibold text-ink">{m.security_cves_card_title()}</h2>
			<p class="tnum m-0 flex flex-wrap items-center gap-x-3 gap-y-1 text-[0.8125rem] text-ink-2">
				<span>{m.security_cves_count_vulnerable()}: <strong class="text-ink">{summary.counts.vulnerable}</strong></span>
				<span>{m.security_cves_count_exploited()}: <strong class="text-ink">{summary.counts.exploited}</strong></span>
				<span>{m.security_cves_count_unknown()}: <strong class="text-ink">{summary.counts.unknown}</strong></span>
				<span>{m.security_cves_count_patched()}: <strong class="text-ink">{summary.counts.patched}</strong></span>
			</p>
		</div>
		{#if top.length === 0}
			<p class="mt-2 text-sm text-ink-2">{m.security_cves_card_clean()}</p>
		{:else}
			<ul class="m-0 mt-2 list-none divide-y divide-line p-0">
				{#each top as f (f.cve_id + f.component)}
					<li class="flex flex-wrap items-center gap-x-2 gap-y-1 py-2 text-sm">
						<a href={f.link} target="_blank" rel="noopener" class="inline-flex min-h-10 items-center font-mono font-semibold text-ink hover:underline sm:min-h-0">{f.cve_id}</a>
						<Plate tone={stateTone(f.state)}>{STATE[f.state]()}</Plate>
						{#if f.exploited}<Plate tone="warning">{m.security_cves_exploited()}</Plate>{/if}
						{#if f.cvss !== null}<Plate tone={severityTone(f.severity)}>{m.security_cves_cvss({ score: f.cvss.toFixed(1) })}</Plate>{/if}
						<span class="min-w-0 text-[0.8125rem] text-ink-2 [overflow-wrap:anywhere]">{f.component} {f.installed_version}</span>
					</li>
				{/each}
			</ul>
		{/if}
		<a href={`/security?tab=cves&target=${targetId}`} class="mt-1 inline-flex min-h-10 items-center text-sm font-semibold text-ink hover:underline">{m.security_cves_card_all()}</a>
	</section>
{/if}
