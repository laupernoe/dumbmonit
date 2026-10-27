<script lang="ts">
	/**
	 * "New from a template": the server's templates as a choice of cards, then
	 * a filter (device type, folder, site, client), a live preview of the
	 * dashboard that results, and one button to create it. Templates are data
	 * served by `GET /api/dashboards/templates`; nothing here knows any of them.
	 * A template with no device to feed it is shown dimmed, with the reason.
	 */
	import { untrack } from 'svelte';
	import { Boxes, Building2, Container, Gauge, Globe, HardDrive, LayoutDashboard, LayoutTemplate, Network, Server } from 'lucide-svelte';
	import type { Icon as LucideIcon } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { listDashboardTemplates, previewDashboardTemplate } from '#lib/api/dashboards.js';
	import type { DashboardPayload, DashboardTemplate, DashboardTemplateFilter, DashboardTemplateList, DashboardTemplatePreview } from '#lib/api/types.js';
	import { kindLabel } from '#lib/kind-label.js';
	import { Button, ErrorNotice, Field, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import DashboardGrid from './DashboardGrid.svelte';
	import WidgetView from './WidgetView.svelte';

	interface Props {
		/** Administrators can share the dashboard they create. */
		mayShare: boolean;
		busy: boolean;
		/** The error of the last attempt to create. */
		error: unknown;
		oncreate: (payload: DashboardPayload) => void;
		oncancel: () => void;
	}

	let { mayShare, busy, error, oncreate, oncancel }: Props = $props();

	const ICONS = { LayoutDashboard, Server, Network, Boxes, HardDrive, Globe, Building2, Container, Gauge } as const;
	const iconOf = (name: string) => (ICONS as Record<string, typeof LucideIcon | undefined>)[name] ?? LayoutTemplate;

	let list = $state<DashboardTemplateList | null>(null);
	let loadError = $state<unknown>(null);
	let chosen = $state<DashboardTemplate | null>(null);

	let kind = $state('');
	let folder = $state('');
	let site = $state('');
	let client = $state('');
	let name = $state('');
	let shared = $state(false);
	let nameEdited = false;

	let preview = $state<DashboardTemplatePreview | null>(null);
	let previewError = $state<unknown>(null);

	$effect(() => {
		const controller = new AbortController();
		listDashboardTemplates(controller.signal)
			.then((found) => (list = found))
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				loadError = cause;
			});
		return () => controller.abort();
	});

	const filter = $derived<DashboardTemplateFilter>({
		kind: kind || undefined,
		folder: folder || undefined,
		site: site ? Number(site) : undefined,
		client: client ? Number(client) : undefined
	});

	// The preview follows the filter; a stale answer never replaces a newer one.
	$effect(() => {
		const template = chosen;
		const wanted = filter;
		if (!template) return;
		const controller = new AbortController();
		previewError = null;
		previewDashboardTemplate(template.id, wanted, controller.signal)
			.then((found) => {
				preview = found;
				untrack(() => {
					if (!nameEdited) name = found.name;
				});
			})
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				previewError = cause;
			});
		return () => controller.abort();
	});

	function choose(template: DashboardTemplate) {
		if (!template.available) return;
		chosen = template;
		preview = null;
		kind = '';
		folder = '';
		site = '';
		client = '';
		shared = false;
		nameEdited = false;
		name = template.title;
	}

	function back() {
		chosen = null;
		preview = null;
	}

	function unavailable(template: DashboardTemplate): string {
		if (template.needs_metric) return m.dash_tpl_unavailable_data();
		if (template.requires.length === 0) return m.dash_tpl_unavailable_any();
		return m.dash_tpl_unavailable_kinds({ kinds: template.requires.map((k) => kindLabel(k)).join(', ') });
	}

	function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!preview || preview.widgets.length === 0 || !name.trim()) return;
		oncreate({
			name: name.trim(),
			shared: shared && mayShare,
			range_secs: preview.range_secs,
			refresh_secs: preview.refresh_secs,
			widgets: preview.widgets
		});
	}

	const kinds = $derived(chosen?.present_kinds ?? []);
	const filters = $derived(list?.filters);
	const matchedNames = $derived(preview ? preview.matched.map((t) => t.name).join(', ') : '');
	const canCreate = $derived(!!preview && preview.widgets.length > 0 && name.trim().length > 0);
	const titleOf = (w: { title: string }) => w.title;
</script>

<Panel title={m.dash_tpl_title()} description={m.dash_tpl_intro()}>
	{#if loadError}
		<ErrorNotice error={loadError} />
	{:else if list === null}
		<Skeleton class="h-40 w-full" />
	{:else if !chosen}
		<ul class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3" aria-label={m.dash_tpl_list_label()}>
			{#each list.templates as template (template.id)}
				{@const Icon = iconOf(template.icon)}
				<li class="flex">
					<button
						type="button"
						class="flex min-h-28 w-full flex-col gap-2 rounded-xl border border-line bg-surface p-4 text-left transition-colors enabled:hover:border-line-strong enabled:hover:bg-surface-2 focus-visible:outline-2 focus-visible:outline-signal disabled:cursor-not-allowed disabled:border-dashed disabled:bg-transparent"
						disabled={!template.available}
						onclick={() => choose(template)}
					>
						<span class="flex items-start gap-3">
							<span class={`flex size-10 shrink-0 items-center justify-center rounded-lg border ${template.available ? 'border-signal/30 bg-signal-soft text-signal-ink' : 'border-line text-ink-3'}`}>
								<Icon class="size-5" aria-hidden="true" />
							</span>
							<span class="min-w-0">
								<span class={`block text-sm font-semibold ${template.available ? 'text-ink' : 'text-ink-2'}`}>{template.title}</span>
								<span class="mt-0.5 block text-[0.8125rem] text-ink-2">{template.description}</span>
							</span>
						</span>
						<span class="mt-auto flex flex-wrap items-center gap-x-2 gap-y-1 pt-1 text-[0.8125rem] text-ink-2">
							{#if template.available}
								<Plate tone="signal" label={m.dash_tpl_devices({ count: template.target_count })} />
								<span>{m.dash_tpl_widgets({ count: template.widget_count })}</span>
							{:else}
								<span>{unavailable(template)}</span>
							{/if}
						</span>
					</button>
				</li>
			{/each}
		</ul>
		<div class="mt-4">
			<Button onclick={oncancel}>{m.dash_cancel()}</Button>
		</div>
	{:else}
		{@const Icon = iconOf(chosen.icon)}
		<form class="flex flex-col gap-5" onsubmit={submit}>
			<div class="flex flex-wrap items-center gap-3">
				<span class="flex size-10 shrink-0 items-center justify-center rounded-lg border border-signal/30 bg-signal-soft text-signal-ink">
					<Icon class="size-5" aria-hidden="true" />
				</span>
				<div class="min-w-0 flex-1">
					<p class="text-base font-semibold text-ink">{chosen.title}</p>
					<p class="text-sm text-ink-2">{chosen.description}</p>
				</div>
				<Button size="sm" onclick={back}>{m.dash_tpl_back()}</Button>
			</div>

			<fieldset class="flex flex-col gap-3">
				<legend class="mb-3 text-sm font-semibold text-ink">{m.dash_tpl_cover_title()}</legend>
				<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
				{#if kinds.length > 1}
					<Field label={m.dash_tpl_filter_kind()} for="tpl-kind">
						<select id="tpl-kind" class="input" bind:value={kind}>
							<option value="">{m.dash_tpl_filter_all_kinds()}</option>
							{#each kinds as k (k.kind)}<option value={k.kind}>{kindLabel(k.kind)} ({k.count})</option>{/each}
						</select>
					</Field>
				{/if}
				{#if filters && filters.folders.length > 0}
					<Field label={m.dash_tpl_filter_folder()} for="tpl-folder">
						<select id="tpl-folder" class="input" bind:value={folder}>
							<option value="">{m.dash_tpl_filter_all_folders()}</option>
							{#each filters.folders as f (f)}<option value={f}>{f}</option>{/each}
						</select>
					</Field>
				{/if}
				{#if filters && filters.sites.length > 0}
					<Field label={m.dash_tpl_filter_site()} for="tpl-site">
						<select id="tpl-site" class="input" bind:value={site}>
							<option value="">{m.dash_tpl_filter_all_sites()}</option>
							{#each filters.sites as s (s.id)}<option value={String(s.id)}>{s.name}</option>{/each}
						</select>
					</Field>
				{/if}
				{#if filters && filters.clients.length > 0}
					<Field label={m.dash_tpl_filter_client()} for="tpl-client">
						<select id="tpl-client" class="input" bind:value={client}>
							<option value="">{m.dash_tpl_filter_all_clients()}</option>
							{#each filters.clients as c (c.id)}<option value={String(c.id)}>{c.name}</option>{/each}
						</select>
					</Field>
				{/if}
				</div>
				<p class="text-[0.8125rem] text-ink-2">{m.dash_tpl_snapshot_note()}</p>
			</fieldset>

			<div class="grid max-w-xl gap-4">
				<Field label={m.dash_name()} for="tpl-name">
					<input
						id="tpl-name"
						class="input"
						maxlength="80"
						required
						autocomplete="off"
						bind:value={name}
						oninput={() => (nameEdited = true)}
					/>
				</Field>
				{#if mayShare}
					<Field label={m.dash_shared()} for="tpl-shared" help={m.dash_shared_help()} inline>
						<Toggle id="tpl-shared" bind:checked={shared} />
					</Field>
				{:else}
					<p class="text-sm text-ink-2">{m.dash_personal_only()}</p>
				{/if}
			</div>

			<section class="flex flex-col gap-3" aria-labelledby="tpl-preview-title">
				<div>
					<h3 id="tpl-preview-title" class="text-sm font-semibold text-ink">{m.dash_tpl_preview_title()}</h3>
					<p class="text-[0.8125rem] text-ink-2">{m.dash_tpl_preview_note()}</p>
				</div>
				{#if previewError}
					<ErrorNotice error={previewError} />
				{:else if preview === null}
					<Skeleton class="h-40 w-full" />
				{:else}
					{@const shown = preview}
					{#if shown.matched_count > 0}
						<p class="text-sm text-ink-2" aria-live="polite">
							{#if shown.matched_count > shown.matched.length}
								{m.dash_tpl_matched_more({ count: shown.matched_count, names: matchedNames })}
							{:else}
								{m.dash_tpl_matched_list({ count: shown.matched_count, names: matchedNames })}
							{/if}
						</p>
					{/if}
					{#if shown.truncated}
						<p class="text-sm text-ink-2">{m.dash_tpl_truncated()}</p>
					{/if}
					{#if shown.widgets.length === 0}
						<p class="rounded-xl border border-dashed border-line px-4 py-6 text-center text-sm text-ink-2">{m.dash_tpl_none_match()}</p>
					{:else}
						<DashboardGrid widgets={shown.widgets} editing={false} {titleOf} onchange={() => {}} onedit={() => {}} onremove={() => {}}>
							{#snippet item(w)}
								<WidgetView widget={w} range={shown.range_secs} tick={0} />
							{/snippet}
						</DashboardGrid>
					{/if}
				{/if}
			</section>

			{#if error}<ErrorNotice {error} />{/if}
			<div class="flex flex-wrap gap-2">
				<Button type="submit" variant="primary" loading={busy} disabled={!canCreate}>{m.dash_tpl_create()}</Button>
				<Button onclick={oncancel}>{m.dash_cancel()}</Button>
			</div>
		</form>
	{/if}
</Panel>
