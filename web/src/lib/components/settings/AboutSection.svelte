<script lang="ts">
	/**
	 * Settings → About: version and the health of the two stores behind the
	 * server. Polled every 15 s while the page is open.
	 */
	import { BookOpen } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getHealth, type ComponentHealth, type Health } from '#lib/api/index.js';
	import { Button, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { upgradeNotes } from '#lib/stores/upgradenotes.svelte.js';
	import { whatsNew } from '#lib/stores/whatsnew.svelte.js';
	import { releaseFor } from '#lib/whatsnew/releases.js';
	import UpdateNotice from '#lib/components/settings/UpdateNotice.svelte';

	let health = $state<Health | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			health = await getHealth(signal);
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
		const timer = setInterval(() => void load(controller.signal), 15_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	// The configuration changes an update asked for (administrators): the list
	// itself is the window mounted by the root layout.
	const upgradeNotesShown = $derived(auth.user?.role === 'admin' && !auth.demo);
	$effect(() => {
		if (upgradeNotesShown) upgradeNotes.load({ quiet: true }).catch(() => {});
	});

	const COMPONENTS: { key: 'database' | 'victoria'; name: () => string; role: () => string }[] = [
		{ key: 'database', name: () => m.settings_about_database(), role: () => m.settings_about_database_role() },
		{ key: 'victoria', name: () => 'VictoriaMetrics', role: () => m.settings_about_victoria_role() }
	];
</script>

<Panel id="about" title={m.settings_about_title()}>
	{#if error}
		<ErrorNotice {error} title={m.settings_about_error_title()} onretry={() => void load()} />
	{:else if loading && !health}
		<Skeleton class="h-5 w-40" />
		<div class="mt-4 grid gap-2"><Skeleton class="h-12 w-full" rows={2} /></div>
	{:else if health}
		<dl class="grid gap-y-3 text-sm sm:grid-cols-[10rem_minmax(0,1fr)] sm:gap-x-6">
			<dt class="font-semibold text-ink">{m.settings_about_version()}</dt>
			<dd class="tnum text-ink-2">
				{health.version}{#if health.build}<span class="text-ink-3"> · {m.settings_about_build()} <span class="font-mono">{health.build}</span></span>{/if}
			</dd>

			{#each COMPONENTS as component (component.key)}
				{@const state: ComponentHealth = health[component.key]}
				<dt class="font-semibold text-ink">{component.name()}</dt>
				<dd class="min-w-0">
					<div class="flex flex-wrap items-center gap-2">
						{#if state.ok}
							<Plate tone="signal" label={m.settings_about_reporting()} />
						{:else}
							<Plate tone="warning" label={m.settings_about_warning()} />
						{/if}
						<span class="text-ink-2">{component.role()}</span>
					</div>
					{#if !state.ok}
						<p class="mt-1 break-words text-warning-ink">{state.error ?? m.settings_about_unreachable()}</p>
						<p class="mt-0.5 text-ink-2">{m.settings_about_check_logs()}</p>
					{/if}
				</dd>
			{/each}
		</dl>
	{/if}

	<UpdateNotice onSeeWhatsNew={() => whatsNew.reopen()} />

	{#if upgradeNotesShown && upgradeNotes.overview}
		<div class="mt-5 flex flex-wrap items-center justify-between gap-x-4 gap-y-3 border-t border-line pt-4 text-sm" data-testid="upgrade-notes-link">
			<div class="min-w-0 flex-1 basis-64">
				<h3 class="font-semibold text-ink">{m.upgrade_about_title()}</h3>
				<p class="mt-0.5 text-ink-2">{m.upgrade_about_text()}</p>
			</div>
			<div class="flex flex-wrap items-center gap-3">
				{#if upgradeNotes.pending > 0}
					<Plate tone="warning" label={m.upgrade_about_pending({ count: upgradeNotes.pending })} />
				{:else}
					<Plate tone="ghost" label={m.upgrade_about_none()} />
				{/if}
				<Button variant="secondary" size="sm" onclick={() => upgradeNotes.show('all')}>{m.upgrade_about_open()}</Button>
			</div>
		</div>
	{/if}

	<div class="mt-5 flex flex-wrap items-center gap-x-4 gap-y-2 border-t border-line pt-4 text-sm">
		<a href="https://dumbmonit.selfhostr.io/docs/" target="_blank" rel="noopener" class="inline-flex items-center gap-1.5 font-medium text-signal-ink hover:underline">
			<BookOpen class="size-4" aria-hidden="true" />
			{m.settings_about_docs()}
		</a>
		{#if health && releaseFor(health.version)}
			<button type="button" class="font-medium text-signal-ink hover:underline" onclick={() => whatsNew.reopen()}>{m.settings_about_whats_new()}</button>
		{/if}
		<span class="text-ink-2">{m.settings_about_license()}</span>
	</div>
</Panel>
