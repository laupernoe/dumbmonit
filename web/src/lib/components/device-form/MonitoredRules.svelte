<script lang="ts">
	/**
	 * "Monitored alerts" on the edit form: every alert rule that reads a metric
	 * this monitor has written, each with a switch. Turning one off stops that
	 * rule's alerts for this monitor only (a per-device override); the data is
	 * still collected. Nothing is sent here: the switches are saved with the
	 * form (see `monitored-rules.ts`), so Cancel drops them.
	 *
	 * Folded by default behind a summary line; the list loads when the form opens.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { ChevronDown } from 'lucide-svelte';
	import { getTargetRules, type TargetRule, type TargetRules } from '#lib/api/index.js';
	import { Button, ErrorNotice, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import { severityTone, severityWord } from '#lib/components/alerts/helpers.js';
	import { isWatched, sortRules, summarise } from './monitored-rules';

	interface Props {
		targetId: number;
		/** Switches the user moved, by rule id: `true` watched, `false` off. */
		choices: Record<number, boolean>;
		/** The loaded rules, for the form to save the choices against. */
		rules: TargetRule[];
	}

	let { targetId, choices = $bindable(), rules = $bindable() }: Props = $props();

	let open = $state(false);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let status = $state<TargetRules['status']>('ok');
	let filter = $state('');

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const result = await getTargetRules(targetId, signal);
			status = result.status;
			rules = result.rules;
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
		return () => controller.abort();
	});

	const sorted = $derived(sortRules(rules));
	const summary = $derived(summarise(rules, choices));
	const needle = $derived(filter.trim().toLowerCase());
	const shown = $derived(needle ? sorted.filter((rule) => rule.name.toLowerCase().includes(needle)) : sorted);

	function setWatched(rule: TargetRule, watched: boolean) {
		choices = { ...choices, [rule.id]: watched };
	}

	function setAll(watched: boolean) {
		const next = { ...choices };
		for (const rule of rules) if (rule.enabled) next[rule.id] = watched;
		choices = next;
	}
</script>

<div class="grid gap-3">
	<div class="graticule pb-3">
		<button
			type="button"
			class="inline-flex min-h-10 flex-wrap items-center gap-x-2 gap-y-1 text-left text-sm font-semibold text-ink-2 hover:text-ink sm:min-h-0"
			aria-expanded={open}
			aria-controls="target-rules"
			onclick={() => (open = !open)}
		>
			<ChevronDown class={`size-4 transition-transform duration-200 ${open ? 'rotate-180' : ''}`} aria-hidden="true" />
			{m.deviceform_rules_title()}
			{#if !loading && !error && status === 'ok'}
				<span class="tnum font-normal text-ink-3">
					{summary.firing > 0
						? m.deviceform_rules_summary_firing({ total: summary.total, off: summary.off, firing: summary.firing })
						: m.deviceform_rules_summary({ total: summary.total, off: summary.off })}
				</span>
			{/if}
		</button>
	</div>

	{#if open}
		<div id="target-rules" class="grid gap-3 rise-in">
			<p class="text-[0.8125rem] leading-relaxed text-ink-2">{m.deviceform_rules_help()}</p>

			{#if loading}
				<Skeleton class="h-10 w-full" rows={3} />
			{:else if error}
				<ErrorNotice {error} title={m.deviceform_rules_load_error()} onretry={() => void load()} />
			{:else if status === 'unavailable'}
				<p class="text-[0.8125rem] text-ink-2">{m.deviceform_rules_unavailable()}</p>
			{:else if status === 'no_data' || rules.length === 0}
				<p class="text-[0.8125rem] text-ink-2">{m.deviceform_rules_no_data()}</p>
			{:else}
				<div class="flex flex-wrap items-center gap-2">
					{#if rules.length > 10}
						<input
							class="input min-w-0 flex-1 sm:max-w-xs"
							type="search"
							placeholder={m.deviceform_rules_search()}
							aria-label={m.deviceform_rules_search()}
							bind:value={filter}
						/>
					{/if}
					<Button variant="ghost" size="sm" onclick={() => setAll(true)}>{m.deviceform_rules_all_on()}</Button>
					<Button variant="ghost" size="sm" onclick={() => setAll(false)}>{m.deviceform_rules_all_off()}</Button>
				</div>

				{#if shown.length === 0}
					<p class="text-[0.8125rem] text-ink-2">{m.deviceform_rules_no_match()}</p>
				{:else}
					<ul class="divide-y divide-line rounded-[var(--radius-card)] border border-line">
						{#each shown as rule (rule.id)}
							<li class={`flex items-center gap-3 px-3 py-2.5 ${rule.enabled ? '' : 'opacity-60'}`}>
								<Toggle
									checked={rule.enabled && isWatched(rule, choices)}
									disabled={!rule.enabled}
									label={m.deviceform_rules_toggle_label({ name: rule.name })}
									onchange={(value) => setWatched(rule, value)}
								/>
								<div class="min-w-0 flex-1">
									<p class="truncate text-sm text-ink" title={rule.description || undefined}>{rule.name}</p>
									{#if !rule.enabled}
										<p class="text-[0.75rem] text-ink-3">
											{m.deviceform_rules_disabled_everywhere()}
											<a href="/alerts#rules" class="font-semibold text-ink-2 underline decoration-line-strong underline-offset-2 hover:text-ink">{m.deviceform_rules_disabled_link()}</a>
										</p>
									{:else if rule.custom_threshold}
										<p class="text-[0.75rem] text-ink-3">{m.deviceform_rules_custom_threshold()}</p>
									{/if}
								</div>
								{#if rule.firing}
									<Plate tone="warning" label={m.deviceform_rules_firing()} pulse />
								{/if}
								<Plate tone={severityTone(rule.severity)} label={severityWord(rule.severity)} />
							</li>
						{/each}
					</ul>
				{/if}
			{/if}
		</div>
	{/if}
</div>
