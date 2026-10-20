<script lang="ts">
	/**
	 * Alerts → Now → Dismissed: the alerts someone dismissed that are still
	 * active, newest first, each with who did it and when, and a way back
	 * (Restore). They are hidden everywhere else and send nothing; once the
	 * problem clears they leave this list by themselves.
	 */
	import type { Alert, AlertRule, Target } from '#lib/api/index.js';
	import { Button, EmptyState } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { dismissals } from '#lib/stores/dismissals.svelte.js';
	import { dismissedByName } from '#lib/alerts-dismiss.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { alertDetail, severityTone, severityWord } from './helpers';
	import { Plate } from '#lib/ui/index.js';
	import { EyeOff, Undo2 } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		alerts: Alert[];
		targets: Map<number, Target>;
		rules: Map<string, AlertRule>;
	}

	let { alerts, targets, rules }: Props = $props();

	let busy = $state<string | null>(null);

	async function restore(alert: Alert) {
		busy = alert.fingerprint;
		try {
			await dismissals.restore([alert.fingerprint]);
		} finally {
			busy = null;
		}
	}

	function whenLine(alert: Alert): string {
		const when = formatRelative(alert.dismissed_at);
		const name = dismissedByName(alert.dismissed_by);
		return name ? m.alerts_dismissed_by({ name, when }) : m.alerts_dismissed_at({ when });
	}
</script>

{#if alerts.length === 0}
	<EmptyState tone="ghost" title={m.alerts_dismissed_empty()} />
{:else}
	<p class="mb-3 text-[0.8125rem] text-ink-2">{m.alerts_dismissed_hint()}</p>
	<ul class="divide-y divide-line rounded-[var(--radius-card)] border border-line bg-surface shadow-lift">
		{#each alerts as alert (alert.fingerprint)}
			{@const target = alert.target_id === null ? undefined : targets.get(alert.target_id)}
			{@const detail = alertDetail(alert, rules.get(alert.rule_uid))}
			<li class="flex min-h-14 items-center gap-2 py-1 pr-1.5 pl-3 sm:pr-2 sm:pl-4">
				<EyeOff class="size-[1.125rem] shrink-0 text-ink-3" aria-hidden="true" />
				<div class="min-w-0 flex-1 py-1.5">
					<p class="flex min-w-0 items-center gap-2">
						<span class="min-w-0 font-semibold text-ink-2 max-sm:line-clamp-2 sm:truncate">{alert.rule_name || alert.rule_uid}</span>
						<Plate tone={severityTone(alert.severity)} label={severityWord(alert.severity)} bare class="shrink-0 max-sm:hidden" />
					</p>
					<p class="mt-0.5 flex min-w-0 items-baseline gap-1.5 text-[0.8125rem] text-ink-2">
						<span class="min-w-0 truncate font-medium">{target?.name ?? (alert.target_id === null ? m.alerts_needs_no_device() : m.alerts_scope_device({ id: alert.target_id }))}</span>
						{#if detail}
							<span class="text-ink-3" aria-hidden="true">·</span>
							<span class="min-w-0 truncate">{detail}</span>
						{/if}
					</p>
					<p class="tnum mt-0.5 text-[0.75rem] text-ink-3" title={formatDateTime(alert.dismissed_at)}>{whenLine(alert)}</p>
				</div>
				{#if auth.canOperate}
					<Button size="sm" variant="ghost" loading={busy === alert.fingerprint} onclick={() => void restore(alert)}>
						<Undo2 class="size-3.5" aria-hidden="true" />
						{m.alerts_dismissed_restore()}
					</Button>
				{/if}
			</li>
		{/each}
	</ul>
{/if}
