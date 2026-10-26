<script lang="ts">
	/**
	 * Create or edit a status page, in plain sections over the full width:
	 * the page itself (title, address, description, theme, history, published),
	 * its services (search to add, then rename, group and order them in a list
	 * that is always visible), its appearance (logo, accent, footer, link,
	 * what the page shows, the banner scene) and the subscribers / domain
	 * settings. Nothing is folded away. A sticky bar keeps Save in reach and
	 * says when something is not saved yet. Saves the page, its logo, then its
	 * services.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { untrack } from 'svelte';
	import { ArrowDown, ArrowUp, Check, X } from 'lucide-svelte';
	import {
		createStatusPage,
		deleteStatusPageLogo,
		setStatusPageItems,
		statusPageLogoUrl,
		toApiError,
		updateStatusPage,
		uploadStatusPageLogo,
		type Channel,
		type StatusPage,
		type StatusPageSeo,
		type StatusPageAccent,
		type StatusPageSceneRotation,
		type StatusPageTheme,
		type Target
	} from '#lib/api/index.js';
	import { Button, Combobox, ErrorNotice, Field, Panel, Plate, Toggle, type ComboOption } from '#lib/ui/index.js';
	import { ACCENTS, accentClass } from './words';
	import { slugify } from './share';
	import { formFingerprint, moveId, savedItems, type ServicePick } from './form-state';
	import DomainField from './DomainField.svelte';
	import GroupPicker from './GroupPicker.svelte';
	import SeoSection from './SeoSection.svelte';
	import { pageUrl } from './seo';
	import { PERIOD_CHOICES } from './period';
	import SceneDefs from './scenes/SceneDefs.svelte';
	import { SCENE_CHOICES, SCENE_COMPONENTS, SCENE_ROTATIONS } from './scenes/registry';

	interface Props {
		/** `null` creates a page. */
		page: StatusPage | null;
		targets: Target[];
		/** Notification channels; the SMTP ones can mail subscribers. */
		channels?: Channel[];
		onsaved: (page: StatusPage) => void;
		oncancel: () => void;
	}

	let { page, targets, channels = [], onsaved, oncancel }: Props = $props();

	// The form seeds itself once from the page it was opened for; the parent
	// re-mounts it for another page.
	const initial = untrack(() => page);

	const SLUG_RULE = /^[a-z0-9-]{2,40}$/;
	const HISTORY_CHOICES = PERIOD_CHOICES;
	const DEFAULT_SCENE = 'paris';

	let title = $state(initial?.title ?? '');
	let slug = $state(initial?.slug ?? '');
	let slugTouched = $state(initial !== null);
	let description = $state(initial?.description ?? '');
	let theme = $state<StatusPageTheme>(initial?.theme ?? 'auto');
	let published = $state(initial?.published ?? false);
	let showDays = $state(initial?.show_uptime_days ?? 90);
	let accent = $state<StatusPageAccent>(initial?.accent ?? 'default');
	// Banner scenes in the order they were ticked; none keeps the plain page. New pages start on one.
	let scenes = $state<string[]>([...(initial?.scenes ?? [DEFAULT_SCENE])]);
	let simple = $state(initial?.simple ?? false);
	let showCalendar = $state(initial?.show_calendar ?? false);
	let showLatency = $state(initial?.show_latency ?? false);
	let sceneRotation = $state<StatusPageSceneRotation>(initial?.scene_rotation ?? 'visit');
	let footerText = $state(initial?.footer_text ?? '');
	let homepageUrl = $state(initial?.homepage_url ?? '');
	let subscribeChannel = $state<number | null>(initial?.subscribe_channel_id ?? null);
	let domain = $state(initial?.domain ?? '');
	// Search-engine settings (shown for a published page) and the monitor folders ticked as a whole.
	let seo = $state<StatusPageSeo>({
		index: false,
		title: '',
		description: '',
		keywords: '',
		canonical: '',
		image: '',
		lang: '',
		site_name: '',
		...initial?.seo
	});
	let groups = $state<string[]>([...(initial?.groups ?? [])]);
	// Public names of the monitors that come in through a ticked group (by monitor id).
	let groupLabels = $state<Record<number, string>>(
		Object.fromEntries((initial?.group_labels ?? []).map((entry) => [entry.target_id, entry.label]))
	);
	let domainError = $state<string | null>(null);
	const smtpChannels = $derived(channels.filter((channel) => channel.kind === 'smtp'));

	// Logo: what is stored, and what this form will do to it on save.
	const MAX_LOGO_BYTES = 256 * 1024;
	const LOGO_TYPES = ['image/png', 'image/jpeg', 'image/webp'];
	let hasStoredLogo = $state(Boolean(initial?.logo_type));
	let pendingLogo = $state<string | null>(null);
	let removeLogo = $state(false);
	let logoError = $state<string | null>(null);
	const logoPreview = $derived(
		pendingLogo ?? (hasStoredLogo && !removeLogo && initial ? statusPageLogoUrl(initial.id, initial.updated_at) : null)
	);

	function pickLogo(event: Event & { currentTarget: HTMLInputElement }) {
		logoError = null;
		const file = event.currentTarget.files?.[0];
		event.currentTarget.value = '';
		if (!file) return;
		if (!LOGO_TYPES.includes(file.type)) {
			logoError = m.status_form_logo_type();
			return;
		}
		if (file.size > MAX_LOGO_BYTES) {
			logoError = m.status_form_logo_size({ kib: MAX_LOGO_BYTES / 1024 });
			return;
		}
		const reader = new FileReader();
		reader.onload = () => {
			pendingLogo = typeof reader.result === 'string' ? reader.result : null;
			removeLogo = false;
		};
		reader.onerror = () => (logoError = m.status_form_logo_read());
		reader.readAsDataURL(file);
	}

	function dropLogo() {
		pendingLogo = null;
		removeLogo = true;
		logoError = null;
	}

	let homepageError = $state<string | null>(null);

	// The scene picker shows the chosen scenes; "Change" opens a searchable, paged grid.
	const SCENES_PER_PAGE = 9;
	let scenePickerOpen = $state(false);
	let sceneQuery = $state('');
	let scenePage = $state(0);
	const sceneMatches = $derived.by(() => {
		const q = sceneQuery.trim().toLowerCase();
		return q ? SCENE_CHOICES.filter((c) => c.label.toLowerCase().includes(q) || c.value.includes(q)) : SCENE_CHOICES;
	});
	const scenePages = $derived(Math.max(1, Math.ceil(sceneMatches.length / SCENES_PER_PAGE)));
	const sceneShown = $derived(sceneMatches.slice(scenePage * SCENES_PER_PAGE, (scenePage + 1) * SCENES_PER_PAGE));
	const chosenScenes = $derived(scenes.flatMap((id) => SCENE_CHOICES.filter((c) => c.value === id)));

	function toggleScene(id: string, on: boolean) {
		scenes = on ? [...scenes.filter((s) => s !== id), id] : scenes.filter((s) => s !== id);
	}

	// Picker state: per target, whether it is shown and how.
	let picks = $state<Map<number, ServicePick>>(
		new Map((initial?.items ?? []).map((item) => [item.target_id, { label: item.label, group: item.group_name }]))
	);
	// Order of appearance = order of the original list, then order of adding.
	let order = $state<number[]>((initial?.items ?? []).map((item) => item.target_id));

	function toggle(target: Target, on: boolean) {
		const next = new Map(picks);
		if (on) {
			next.set(target.id, { label: target.name, group: '' });
			order = [...order.filter((id) => id !== target.id), target.id];
		} else {
			next.delete(target.id);
			order = order.filter((id) => id !== target.id);
		}
		picks = next;
	}

	function setPick(id: number, patch: Partial<ServicePick>) {
		const current = picks.get(id);
		if (!current) return;
		const next = new Map(picks);
		next.set(id, { ...current, ...patch });
		picks = next;
	}

	function toggleGroup(name: string, on: boolean) {
		groups = on ? [...groups.filter((g) => g !== name), name] : groups.filter((g) => g !== name);
	}

	function move(id: number, delta: -1 | 1) {
		order = moveId(order, id, delta);
	}

	const pickedIds = $derived(new Set(order));
	// Monitors that a ticked folder brings in and that are not ticked one by one.
	const viaGroups = $derived(targets.filter((t) => t.group_name && groups.includes(t.group_name) && !pickedIds.has(t.id)));
	// Public names of what the page lists, for the AI prompt.
	const serviceNames = $derived([
		...order.map((id) => picks.get(id)?.label.trim() || targetById.get(id)?.name || '').filter(Boolean),
		...viaGroups.map((t) => groupLabels[t.id]?.trim() || t.name)
	]);
	const seoUrl = $derived(pageUrl(typeof window === 'undefined' ? '' : window.location.origin, slug, domain));
	const seoLogo = $derived(initial?.logo_type && typeof window !== 'undefined' ? `${window.location.origin}/api/public/status/${slug}/logo` : '');
	const groupsSeen = $derived([...new Set([...picks.values()].map((p) => p.group).filter(Boolean))]);
	const targetById = $derived(new Map(targets.map((t) => [t.id, t])));
	const sortedTargets = $derived([...targets].sort((a, b) => a.name.localeCompare(b.name)));
	// Offered to the combobox: filtered on name, type and folder, grouped by folder.
	const serviceOptions = $derived<ComboOption[]>(
		sortedTargets.map((t) => ({ value: String(t.id), label: t.name, hint: t.kind, group: t.group_name }))
	);
	const ICON_BTN =
		'inline-flex size-10 items-center justify-center rounded-md text-ink-2 hover:bg-surface-2 hover:text-ink disabled:opacity-40 disabled:hover:bg-transparent focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ink';

	// "Unsaved changes": everything that would be sent, compared with how the form opened.
	const fingerprint = $derived(
		formFingerprint({
			title,
			slug,
			description,
			theme,
			published,
			showDays,
			accent,
			scenes,
			sceneRotation,
			simple,
			showCalendar,
			showLatency,
			footerText,
			homepageUrl,
			subscribeChannel,
			domain,
			seo,
			groups,
			groupLabels,
			items: savedItems(order, picks),
			logo: pendingLogo !== null || (removeLogo && hasStoredLogo)
		})
	);
	const openedAs = untrack(() => fingerprint);
	const dirty = $derived(fingerprint !== openedAs);

	let titleError = $state<string | null>(null);
	let slugError = $state<string | null>(null);
	let saving = $state(false);
	let error = $state<unknown>(null);

	// Leaving the browser tab with unsaved edits asks first.
	$effect(() => {
		if (!dirty || saving) return;
		const warn = (event: BeforeUnloadEvent) => event.preventDefault();
		window.addEventListener('beforeunload', warn);
		return () => window.removeEventListener('beforeunload', warn);
	});

	function onTitleInput() {
		titleError = null;
		if (!slugTouched) slug = slugify(title);
	}

	/** Brings the first refused field into view, since Save sits far below some of them. */
	function focusFirstError() {
		const order: [string | null, string][] = [
			[titleError, 'title'],
			[slugError, 'slug'],
			[homepageError, 'homepage'],
			[domainError, 'domain']
		];
		const first = order.find(([message]) => message);
		if (!first) return;
		const field = document.getElementById(`${idPrefix}-${first[1]}`);
		field?.scrollIntoView({ block: 'center' });
		field?.focus({ preventScroll: true });
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		titleError = title.trim() ? null : m.status_form_title_required();
		slugError = SLUG_RULE.test(slug) ? null : m.status_form_slug_rule();
		homepageError = homepageUrl.trim() === '' || /^https?:\/\/[^\s/]+/i.test(homepageUrl.trim()) ? null : m.status_form_homepage_rule();
		// The server has the full rule; this only catches the usual slips early.
		const host = domain.trim().replace(/^https?:\/\//i, '').replace(/\/$/, '');
		domainError =
			host === '' || /^[a-z0-9.-]+$/i.test(host) ? null : m.status_form_domain_rule();
		if (titleError || slugError || homepageError || domainError) {
			focusFirstError();
			return;
		}

		saving = true;
		try {
			const payload = {
				title: title.trim(),
				slug,
				description: description.trim(),
				theme,
				published,
				show_uptime_days: showDays,
				accent,
				scenes,
				scene_rotation: sceneRotation,
				simple,
				show_calendar: showCalendar,
				show_latency: showLatency,
				footer_text: footerText.trim(),
				homepage_url: homepageUrl.trim(),
				subscribe_channel_id: subscribeChannel,
				domain: host === '' ? null : host,
				seo,
				groups,
				group_labels: Object.entries(groupLabels)
					.map(([id, label]) => ({ target_id: Number(id), label: label.trim() }))
					.filter((entry) => entry.label !== '' && viaGroups.some((t) => t.id === entry.target_id))
			};
			let saved = page ? await updateStatusPage(page.id, payload) : await createStatusPage(payload);
			if (pendingLogo) {
				await uploadStatusPageLogo(saved.id, pendingLogo);
				saved = { ...saved, logo_type: 'image' };
			} else if (removeLogo && hasStoredLogo) {
				await deleteStatusPageLogo(saved.id);
				saved = { ...saved, logo_type: null };
			}
			const stored = await setStatusPageItems(saved.id, savedItems(order, picks));
			onsaved({ ...saved, items: stored });
		} catch (cause) {
			// A refused domain (taken, invalid, DumbMonit's own address) belongs under its field.
			const api = toApiError(cause);
			if ((api.status === 400 || api.status === 409) && /domain|host name|IP address|punycode|DumbMonit itself/i.test(api.message)) {
				domainError = api.message;
				focusFirstError();
			} else {
				error = cause;
			}
		} finally {
			saving = false;
		}
	}

	const idPrefix = $derived(page ? `sp-${page.id}` : 'sp-new');
	const SUBHEAD = 'text-sm font-semibold text-ink';
</script>

<form class="grid gap-4" onsubmit={submit} novalidate>
	<!-- 1. The page: what visitors read first, and where they find it. -->
	<Panel title={m.status_edit_section_page()} description={m.status_edit_section_page_help()}>
		<div class="grid items-start gap-x-5 gap-y-4 md:grid-cols-2 xl:grid-cols-3">
			<Field label={m.status_form_title()} for="{idPrefix}-title" error={titleError} required>
				<input
					id="{idPrefix}-title"
					type="text"
					class="input"
					bind:value={title}
					oninput={onTitleInput}
					placeholder={m.status_form_title_placeholder()}
					maxlength="120"
					disabled={saving}
					aria-invalid={titleError ? 'true' : undefined}
				/>
			</Field>
			<Field label={m.status_form_address()} for="{idPrefix}-slug" error={slugError} help={slug ? m.status_form_address_url({ url: `/s/${slug}` }) : m.status_form_address_help()} required>
				<input
					id="{idPrefix}-slug"
					type="text"
					class="input font-mono"
					bind:value={slug}
					oninput={() => {
						slugTouched = true;
						slugError = null;
					}}
					placeholder="home-lab"
					maxlength="40"
					autocomplete="off"
					spellcheck="false"
					disabled={saving}
					aria-invalid={slugError ? 'true' : undefined}
				/>
			</Field>
			<div class="grid gap-1.5">
				<span class={SUBHEAD} aria-hidden="true">{m.status_edit_visibility()}</span>
				<div class="flex min-h-10 items-center justify-between gap-3 rounded-lg border border-line px-3 py-1.5">
					<div class="min-w-0">
						<label for="{idPrefix}-published" class="block text-sm font-semibold text-ink">{m.status_form_published()}</label>
						<p class="text-[0.8125rem] text-ink-2">{published ? m.status_form_published_on() : m.status_form_published_off()}</p>
					</div>
					<Toggle id="{idPrefix}-published" bind:checked={published} disabled={saving} label={m.status_form_published()} />
				</div>
			</div>
			<Field label={m.status_form_description()} for="{idPrefix}-description" help={m.status_form_description_help()}>
				<input id="{idPrefix}-description" type="text" class="input" bind:value={description} maxlength="1000" placeholder={m.status_form_description_placeholder()} disabled={saving} />
			</Field>
			<Field label={m.status_form_theme()} for="{idPrefix}-theme" help={m.status_edit_theme_help()}>
				<select id="{idPrefix}-theme" class="input" bind:value={theme} disabled={saving}>
					<option value="auto">{m.status_form_theme_auto()}</option>
					<option value="light">{m.status_form_theme_day()}</option>
					<option value="dark">{m.status_form_theme_night()}</option>
				</select>
			</Field>
			<Field label={m.status_form_history()} for="{idPrefix}-days" help={m.status_form_history_help()}>
				<select id="{idPrefix}-days" class="input" bind:value={showDays} disabled={saving}>
					{#each HISTORY_CHOICES as choice (choice)}
						<option value={choice}>{choice === 1 ? m.status_period_24h() : m.status_share_days({ days: choice })}</option>
					{/each}
				</select>
			</Field>
		</div>
	</Panel>

	<!-- 2. Services: search to add on the left, the list as visitors will see it on the right. -->
	<Panel title={m.status_form_services()} description={m.status_edit_section_services_help()}>
		{#snippet aside()}
			{#if order.length + viaGroups.length > 0}
				<span class="text-sm text-ink-2 tnum">{m.status_form_services_count({ count: order.length + viaGroups.length })}</span>
			{/if}
		{/snippet}
		{#if sortedTargets.length === 0}
			<p class="ghost-cell rounded-lg border border-dashed border-line px-4 py-6 text-center text-sm text-ink-2">{m.status_form_services_empty()}</p>
		{:else}
			<fieldset class="grid items-start gap-x-6 gap-y-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,2fr)]" disabled={saving}>
				<legend class="sr-only">{m.status_form_services()}</legend>
				<div class="grid gap-1.5">
					<span class={SUBHEAD}>{m.status_edit_services_add()}</span>
					<GroupPicker
						id="{idPrefix}-folders"
						{targets}
						{groups}
						picked={pickedIds}
						ontogglegroup={toggleGroup}
						ontoggletarget={toggle}
						disabled={saving}
					/>
					<Combobox
						id="{idPrefix}-services"
						options={serviceOptions}
						selected={order.map(String)}
						ontoggle={(value, on) => {
							const target = targetById.get(Number(value));
							if (target) toggle(target, on);
						}}
						label={m.status_form_services_search()}
						emptyText={m.status_form_services_no_match()}
						ungroupedLabel={m.status_form_services_no_folder()}
					/>
					<p class="text-[0.8125rem] text-ink-2">{m.status_edit_services_add_help()}</p>
				</div>

				<div class="grid min-w-0 gap-1.5">
					<span class={SUBHEAD} id="{idPrefix}-list-label">{m.status_edit_services_list()}</span>
					{#if order.length === 0 && groups.length === 0}
						<p class="ghost-cell rounded-lg border border-dashed border-line px-4 py-6 text-center text-sm text-ink-2">{m.status_edit_services_none()}</p>
					{:else if order.length > 0}
						<div class="overflow-hidden rounded-[var(--radius-card)] border border-line">
							<div class="hidden gap-2 border-b border-line bg-canvas-deep px-3 py-1.5 text-[0.75rem] text-ink-2 md:grid md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,0.75fr)_7.5rem]" aria-hidden="true">
								<span class="pl-7">{m.status_services_private_name()}</span>
								<span>{m.status_services_public_name()}</span>
								<span>{m.status_edit_services_col_group()}</span>
								<span class="text-right">{m.status_edit_services_col_order()}</span>
							</div>
							<ol class="divide-y divide-line" aria-labelledby="{idPrefix}-list-label">
								{#each order as id, index (id)}
									{@const target = targetById.get(id)}
									{@const pick = picks.get(id)}
									{#if target && pick}
										<li class="grid gap-2 px-3 py-2 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,0.75fr)_7.5rem] md:items-center">
											<div class="flex min-w-0 items-center gap-2 text-sm text-ink">
												<span class="w-5 shrink-0 text-right text-[0.75rem] tnum text-ink-3">{index + 1}</span>
												<span class="grid min-w-0">
													<span class="truncate text-ink-2" title={m.status_services_private_hint()}>{target.name}</span>
													<span class="truncate text-[0.75rem] text-ink-3">{target.kind} · {m.status_services_private_hint()}</span>
												</span>
											</div>
											<div class="grid gap-1">
												<input
													type="text"
													class="input"
													value={pick.label}
													oninput={(event) => setPick(id, { label: event.currentTarget.value })}
													placeholder={target.name}
													maxlength="80"
													aria-label={m.status_form_pick_label_aria({ name: target.name })}
												/>
												{#if !pick.label.trim()}
													<p class="text-[0.75rem] text-warning">{m.status_services_public_empty({ name: target.name })}</p>
												{/if}
											</div>
											<input
												type="text"
												class="input"
												value={pick.group}
												oninput={(event) => setPick(id, { group: event.currentTarget.value })}
												placeholder={m.status_form_pick_group()}
												maxlength="60"
												list="{idPrefix}-groups"
												aria-label={m.status_form_pick_group_aria({ name: target.name })}
											/>
											<div class="flex justify-end">
												<button type="button" class={ICON_BTN} onclick={() => move(id, -1)} disabled={index === 0} aria-label={m.status_form_pick_up_aria({ name: target.name })} title={m.status_form_up()}><ArrowUp class="size-4" aria-hidden="true" /></button>
												<button type="button" class={ICON_BTN} onclick={() => move(id, 1)} disabled={index === order.length - 1} aria-label={m.status_form_pick_down_aria({ name: target.name })} title={m.status_form_down()}><ArrowDown class="size-4" aria-hidden="true" /></button>
												<button type="button" class={ICON_BTN} onclick={() => toggle(target, false)} aria-label={m.status_form_pick_remove_aria({ name: target.name })} title={m.status_form_logo_remove()}><X class="size-4" aria-hidden="true" /></button>
											</div>
										</li>
									{/if}
								{/each}
							</ol>
						</div>
						<datalist id="{idPrefix}-groups">
							{#each groupsSeen as group (group)}<option value={group}></option>{/each}
						</datalist>
					{/if}
					{#if viaGroups.length > 0}
						<div class="grid gap-1.5">
							<span class={SUBHEAD}>{m.status_services_from_groups()}</span>
							<ul class="divide-y divide-line overflow-hidden rounded-[var(--radius-card)] border border-line">
								{#each viaGroups as target (target.id)}
									<li class="grid gap-2 px-3 py-2 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] md:items-start">
										<span class="grid min-w-0">
											<span class="truncate text-sm text-ink-2" title={m.status_services_private_hint()}>{target.name}</span>
											<span class="truncate text-[0.75rem] text-ink-3">{target.group_name} · {m.status_services_private_hint()}</span>
										</span>
										<div class="grid gap-1">
											<input
												type="text"
												class="input"
												value={groupLabels[target.id] ?? ''}
												oninput={(event) => (groupLabels = { ...groupLabels, [target.id]: event.currentTarget.value })}
												placeholder={target.name}
												maxlength="80"
												aria-label={m.status_form_pick_label_aria({ name: target.name })}
											/>
											{#if !(groupLabels[target.id] ?? '').trim()}
												<p class="text-[0.75rem] text-warning">{m.status_services_public_empty({ name: target.name })}</p>
											{/if}
										</div>
									</li>
								{/each}
							</ul>
						</div>
					{/if}
					{#if groups.length > 0}
						<ul class="grid gap-1 text-[0.8125rem] text-ink-2" aria-label={m.status_groups_dynamic_aria()}>
							{#each groups as group (group)}
								<li>{m.status_groups_dynamic({ group, count: targets.filter((t) => t.group_name === group).length })}</li>
							{/each}
						</ul>
					{/if}
				</div>
			</fieldset>
		{/if}
	</Panel>

	<!-- 3. Appearance: branding on the left; the scene and what the page shows on the right. -->
	<Panel title={m.status_edit_section_appearance()} description={m.status_edit_section_appearance_help()}>
		<div class="grid items-start gap-x-8 gap-y-6 lg:grid-cols-2">
			<fieldset class="grid gap-5" disabled={saving}>
				<legend class="sr-only">{m.status_form_look()}</legend>
				<div class="grid content-start gap-1.5">
					<span class={SUBHEAD} id="{idPrefix}-logo-label">{m.status_form_logo()}</span>
					<div class="flex items-center gap-3">
						<div class="flex size-14 shrink-0 items-center justify-center overflow-hidden rounded-lg border border-dashed border-line-strong bg-canvas-deep">
							{#if logoPreview}
								<img src={logoPreview} alt={m.status_form_logo_preview()} class="size-full object-contain" />
							{:else}
								<span class="text-[0.6875rem] text-ink-3">{m.status_form_logo_none()}</span>
							{/if}
						</div>
						<div class="flex flex-wrap items-center gap-1.5">
							<label class="inline-flex min-h-10 cursor-pointer items-center rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-semibold text-ink hover:bg-surface-2 focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-ink">
								{logoPreview ? m.status_form_logo_replace() : m.status_form_logo_upload()}
								<input type="file" accept="image/png,image/jpeg,image/webp" class="sr-only" onchange={pickLogo} aria-labelledby="{idPrefix}-logo-label" />
							</label>
							{#if logoPreview}
								<Button variant="ghost" size="sm" onclick={dropLogo}>{m.status_form_logo_remove()}</Button>
							{/if}
						</div>
					</div>
					<p class="text-[0.8125rem] text-ink-2">{m.status_form_logo_help()}</p>
					{#if logoError}<p class="text-[0.8125rem] text-warning-ink" role="alert">{logoError}</p>{/if}
				</div>
				<div class="grid content-start gap-1.5">
					<span class={SUBHEAD} id="{idPrefix}-accent-label">{m.status_form_accent()}</span>
					<div class="flex flex-wrap gap-2" role="radiogroup" aria-labelledby="{idPrefix}-accent-label">
						{#each ACCENTS as option (option.value)}
							<label class={`inline-flex min-h-10 cursor-pointer items-center gap-2 rounded-md border px-2.5 py-1.5 text-sm focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-ink ${accent === option.value ? 'border-ink bg-surface-2 text-ink' : 'border-line text-ink-2 hover:text-ink'} ${accentClass(option.value)}`}>
								<input type="radio" class="sr-only" name="{idPrefix}-accent" value={option.value} bind:group={accent} />
								<span class="size-3.5 rounded-full bg-accent" aria-hidden="true"></span>
								{option.label}
							</label>
						{/each}
					</div>
					<p class="text-[0.8125rem] text-ink-2">{m.status_form_accent_help()}</p>
				</div>
				<div class="grid items-start gap-4 sm:grid-cols-2">
					<Field label={m.status_form_homepage()} for="{idPrefix}-homepage" error={homepageError} help={m.status_form_homepage_help()}>
						<input id="{idPrefix}-homepage" type="url" class="input" bind:value={homepageUrl} maxlength="300" placeholder="https://example.org" disabled={saving} oninput={() => (homepageError = null)} />
					</Field>
					<Field label={m.status_form_footer()} for="{idPrefix}-footer" help={m.status_form_footer_help()}>
						<textarea id="{idPrefix}-footer" class="input min-h-10" bind:value={footerText} maxlength="280" rows="1" disabled={saving}></textarea>
					</Field>
				</div>
			</fieldset>

			<div class="grid min-w-0 gap-6 lg:border-l lg:border-line lg:pl-8">
				<fieldset class="grid min-w-0 gap-3" disabled={saving || simple}>
					<legend class={`${SUBHEAD} mb-1.5`}>{m.status_form_scene()}</legend>
					<p class="text-[0.8125rem] text-ink-2">{m.status_form_scene_help()}</p>
					{#if simple}
						<p class="text-[0.8125rem] text-advisory-ink">{m.status_edit_scene_simple()}</p>
					{/if}
					<SceneDefs />
					<div class="flex flex-wrap items-center gap-3">
						{#if chosenScenes.length === 0}
							<p class="text-sm text-ink-2">{m.status_form_scene_none()}</p>
						{/if}
						{#each chosenScenes as choice, index (choice.value)}
							{@const Preview = SCENE_COMPONENTS[choice.value]}
							<span class="grid w-32 gap-1 rounded-lg border border-ink bg-surface-2 p-1.5">
								<span class="relative block h-14 overflow-hidden rounded-md border border-line" aria-hidden="true"><Preview /></span>
								<span class="flex items-center justify-between gap-1 px-0.5 text-sm text-ink">
									<span class="truncate">{choice.label}</span>
									{#if chosenScenes.length > 1}<span class="text-[0.6875rem] font-semibold text-ink-2">{index + 1}</span>{/if}
								</span>
							</span>
						{/each}
						<Button variant="secondary" size="sm" onclick={() => (scenePickerOpen = !scenePickerOpen)} aria-expanded={scenePickerOpen}>
							{scenePickerOpen ? m.status_form_scene_done() : m.status_form_scene_change()}
						</Button>
					</div>
					{#if scenePickerOpen}
						<input
							type="search"
							class="input"
							bind:value={sceneQuery}
							oninput={() => (scenePage = 0)}
							placeholder={m.status_form_scene_search()}
							aria-label={m.status_form_scene_search()}
							autocomplete="off"
						/>
						<div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
							{#each sceneShown as choice (choice.value)}
								{@const Preview = SCENE_COMPONENTS[choice.value]}
								{@const rank = scenes.indexOf(choice.value)}
								<label
									class={`group relative grid cursor-pointer gap-1.5 rounded-lg border p-1.5 focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-ink ${rank >= 0 ? 'border-ink bg-surface-2' : 'border-line hover:border-line-strong'}`}
								>
									<input
										type="checkbox"
										class="sr-only"
										checked={rank >= 0}
										onchange={(event) => toggleScene(choice.value, event.currentTarget.checked)}
									/>
									<span class="relative block h-20 overflow-hidden rounded-md border border-line" aria-hidden="true">
										<Preview />
									</span>
									<span class="flex items-center justify-between gap-2 px-0.5 text-sm text-ink">
										{choice.label}
										{#if rank >= 0}
											<span class="inline-flex min-w-5 items-center justify-center rounded-full bg-ink px-1.5 text-[0.6875rem] font-semibold text-canvas" aria-label={scenes.length > 1 ? m.status_form_scene_position({ position: rank + 1 }) : m.status_form_scene_selected()}>
												{scenes.length > 1 ? rank + 1 : '✓'}
											</span>
										{/if}
									</span>
								</label>
							{:else}
								<p class="col-span-full text-sm text-ink-2">{m.status_form_scene_no_match()}</p>
							{/each}
						</div>
						{#if scenePages > 1}
							<div class="flex items-center justify-between gap-2">
								<Button variant="ghost" size="sm" onclick={() => (scenePage -= 1)} disabled={scenePage === 0}>{m.status_form_scene_prev()}</Button>
								<span class="text-sm text-ink-2" aria-live="polite">{m.status_form_scene_page({ page: scenePage + 1, pages: scenePages })}</span>
								<Button variant="ghost" size="sm" onclick={() => (scenePage += 1)} disabled={scenePage >= scenePages - 1}>{m.status_form_scene_next()}</Button>
							</div>
						{/if}
					{/if}
					{#if scenes.length > 1}
						<Field label={m.status_form_rotation()} for="{idPrefix}-rotation" help={m.status_form_rotation_help()}>
							<select id="{idPrefix}-rotation" class="input" bind:value={sceneRotation} disabled={saving}>
								{#each SCENE_ROTATIONS as option (option.value)}
									<option value={option.value}>{option.label}</option>
								{/each}
							</select>
						</Field>
					{/if}
				</fieldset>
				<div class="grid gap-1.5">
					<span class={SUBHEAD} aria-hidden="true">{m.status_edit_appearance_shows()}</span>
					<ul class="divide-y divide-line rounded-lg border border-line">
						{#each [{ key: 'simple', label: m.status_form_simple(), help: m.status_form_simple_help() }, { key: 'calendar', label: m.status_form_calendar(), help: m.status_form_calendar_help() }, { key: 'latency', label: m.status_form_latency(), help: m.status_form_latency_help() }] as opt (opt.key)}
							<li class="flex min-h-12 items-center justify-between gap-4 px-3 py-2">
								<div class="min-w-0">
									<label for="{idPrefix}-{opt.key}" class="block text-sm text-ink">{opt.label}</label>
									<p class="text-[0.8125rem] text-ink-2">{opt.help}</p>
								</div>
								{#if opt.key === 'simple'}
									<Toggle id="{idPrefix}-simple" bind:checked={simple} disabled={saving} label={opt.label} />
								{:else if opt.key === 'calendar'}
									<Toggle id="{idPrefix}-calendar" bind:checked={showCalendar} disabled={saving} label={opt.label} />
								{:else}
									<Toggle id="{idPrefix}-latency" bind:checked={showLatency} disabled={saving} label={opt.label} />
								{/if}
							</li>
						{/each}
					</ul>
				</div>
			</div>
		</div>
	</Panel>

	<!-- 4. Search engines: only for a published page. -->
	{#if published}
		<SeoSection
			{idPrefix}
			bind:seo
			name={title}
			{description}
			url={seoUrl}
			logo={seoLogo}
			services={serviceNames}
			disabled={saving}
		/>
	{/if}

	<!-- 5. Email updates and the page's own address. -->
	<Panel title={m.status_form_section_connect()} description={m.status_edit_section_connect_help()}>
		<div class="grid items-start gap-x-8 gap-y-4 lg:grid-cols-2">
			<Field
				label={m.status_form_subscribers()}
				for="{idPrefix}-subscribe"
				help={smtpChannels.length === 0 ? m.status_form_subscribers_no_channel() : m.status_form_subscribers_help()}
			>
				<select
					id="{idPrefix}-subscribe"
					class="input"
					value={subscribeChannel === null ? '' : String(subscribeChannel)}
					onchange={(event) => (subscribeChannel = event.currentTarget.value === '' ? null : Number(event.currentTarget.value))}
					disabled={saving || (smtpChannels.length === 0 && subscribeChannel === null)}
				>
					<option value="">{m.status_form_subscribers_off()}</option>
					{#each smtpChannels as channel (channel.id)}
						<option value={String(channel.id)}>{channel.enabled ? m.status_form_subscribers_via({ name: channel.name }) : m.status_form_subscribers_via_disabled({ name: channel.name })}</option>
					{/each}
				</select>
			</Field>
			<DomainField
				id="{idPrefix}-domain"
				bind:value={domain}
				saved={initial?.domain ?? null}
				error={domainError}
				disabled={saving}
				oninput={() => (domainError = null)}
			/>
		</div>
	</Panel>

	{#if error}
		<ErrorNotice {error} title={page ? m.status_form_save_error() : m.status_form_create_error()} />
	{/if}

	<!-- Save stays in reach: with something to save, the bar sticks to the bottom of the screen (above the phone tab bar). -->
	<div class={dirty || page === null ? 'sticky bottom-[calc(var(--tabbar-h)+var(--safe-bottom)+0.5rem)] z-20 sm:bottom-3' : ''}>
		<div class={`flex flex-wrap items-center gap-x-3 gap-y-2 rounded-[var(--radius-card)] border bg-surface px-4 py-2.5 shadow-float ${dirty ? 'border-line-strong' : 'border-line'}`}>
			<div class="min-w-0 flex-1 max-sm:basis-full" aria-live="polite">
				{#if page === null}
					<span class="text-sm text-ink-2">{m.status_edit_bar_new()}</span>
				{:else if dirty}
					<Plate tone="advisory" label={m.status_edit_bar_unsaved()} class="whitespace-nowrap" />
				{:else}
					<span class="text-sm text-ink-2">{m.status_edit_bar_clean()}</span>
				{/if}
			</div>
			<Button variant="ghost" onclick={oncancel} disabled={saving} class="max-sm:ml-auto">
				<X class="size-4" aria-hidden="true" />
				{m.status_incidents_cancel()}
			</Button>
			<Button type="submit" variant="primary" loading={saving}>
				<Check class="size-4" aria-hidden="true" />
				{page ? m.status_form_save() : m.status_form_create()}
			</Button>
		</div>
	</div>
</form>
