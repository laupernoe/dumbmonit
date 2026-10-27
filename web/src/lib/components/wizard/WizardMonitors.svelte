<script lang="ts">
	/**
	 * Setup wizard, step "Add your first monitors": the network scan in place,
	 * or the import and add-by-hand pages in a new tab — the wizard stays where
	 * it is, and the count below follows what was added.
	 */
	import { ExternalLink, Network, Radar, Upload, Plus } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button } from '#lib/ui/index.js';
	import AdDeployWizard from '#lib/components/settings/AdDeployWizard.svelte';
	import Discovery from '#lib/components/device-form/Discovery.svelte';

	let scanning = $state(false);
	let refreshing = $state(false);

	async function refresh() {
		refreshing = true;
		await alertsStore.refresh();
		refreshing = false;
	}

	const card =
		'flex min-h-24 flex-col gap-1.5 rounded-[var(--radius-card)] border border-line bg-surface p-4 text-left transition-colors hover:border-line-strong';
</script>

<div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
	<button type="button" class={`${card} ${scanning ? '!border-ink' : ''}`} aria-expanded={scanning} aria-controls="wizard-scan" onclick={() => (scanning = !scanning)}>
		<Radar class="size-5 text-ink-2" aria-hidden="true" />
		<span class="font-semibold text-ink">{m.wizard_monitors_scan()}</span>
		<span class="text-[0.8125rem] text-ink-2">{m.wizard_monitors_scan_text()}</span>
	</button>
	<a class={card} href="/targets/new" target="_blank" rel="noopener">
		<Upload class="size-5 text-ink-2" aria-hidden="true" />
		<span class="inline-flex items-center gap-1 font-semibold text-ink">{m.wizard_monitors_import()}<ExternalLink class="size-3.5 text-ink-3" aria-hidden="true" /></span>
		<span class="text-[0.8125rem] text-ink-2">{m.wizard_monitors_import_text()}</span>
	</a>
	<a class={card} href="/targets/new" target="_blank" rel="noopener">
		<Plus class="size-5 text-ink-2" aria-hidden="true" />
		<span class="inline-flex items-center gap-1 font-semibold text-ink">{m.wizard_monitors_manual()}<ExternalLink class="size-3.5 text-ink-3" aria-hidden="true" /></span>
		<span class="text-[0.8125rem] text-ink-2">{m.wizard_monitors_manual_text()}</span>
	</a>
	{#if auth.isAdmin}
		<!-- Active Directory: the group-policy deployment flow, not a plain form. -->
		<div class={`${card} justify-between`}>
			<Network class="size-5 text-ink-2" aria-hidden="true" />
			<span class="font-semibold text-ink">{m.wizard_monitors_ad()}</span>
			<span class="text-[0.8125rem] text-ink-2">{m.wizard_monitors_ad_text()}</span>
			<AdDeployWizard class="mt-1 self-start" />
		</div>
	{/if}
</div>
<p class="mt-2 text-[0.8125rem] text-ink-2">{m.wizard_monitors_new_tab()}</p>

{#if scanning}
	<div id="wizard-scan" class="mt-4 min-w-0">
		<Discovery />
	</div>
{/if}

<div class="mt-4 flex flex-wrap items-center gap-3" aria-live="polite">
	<span class="text-sm text-ink">{m.wizard_monitors_count({ count: alertsStore.targets.length })}</span>
	<Button size="sm" variant="ghost" loading={refreshing} onclick={refresh}>{m.wizard_monitors_refresh()}</Button>
</div>
