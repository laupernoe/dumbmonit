<script lang="ts">
	/**
	 * Create or edit a status page, inline: title, slug (suggested from the
	 * title until typed by hand), description, theme, history depth, published
	 * toggle, the look (logo, accent, footer, link to the organisation's site), the
	 * optional banner scene(s),
	 * email subscription, the optional public domain, and the service picker — tick devices, name them for
	 * the public, group them. Saves the page, its logo, then its services.
	 */
	import { untrack } from 'svelte';
	import { Check, X } from 'lucide-svelte';
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
		type StatusPageAccent,
		type StatusPageItemPayload,
		type StatusPageSceneRotation,
		type StatusPageTheme,
		type Target
	} from '#lib/api/index.js';
	import { Button, ErrorNotice, Field, Toggle } from '#lib/ui/index.js';
	import { ACCENTS, accentClass, slugify } from './words';
	import DomainField from './DomainField.svelte';
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
	const HISTORY_CHOICES = [30, 60, 90];

	let title = $state(initial?.title ?? '');
	let slug = $state(initial?.slug ?? '');
	let slugTouched = $state(initial !== null);
	let description = $state(initial?.description ?? '');
	let theme = $state<StatusPageTheme>(initial?.theme ?? 'auto');
	let published = $state(initial?.published ?? false);
	let showDays = $state(initial?.show_uptime_days ?? 90);
	let accent = $state<StatusPageAccent>(initial?.accent ?? 'default');
	// Banner scenes in the order they were ticked; none (the default) keeps the plain page.
	let scenes = $state<string[]>([...(initial?.scenes ?? [])]);
	let sceneRotation = $state<StatusPageSceneRotation>(initial?.scene_rotation ?? 'visit');
	let footerText = $state(initial?.footer_text ?? '');
	let homepageUrl = $state(initial?.homepage_url ?? '');
	let subscribeChannel = $state<number | null>(initial?.subscribe_channel_id ?? null);
	let domain = $state(initial?.domain ?? '');
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
			logoError = 'Use a PNG, JPEG or WebP image.';
			return;
		}
		if (file.size > MAX_LOGO_BYTES) {
			logoError = `The logo is limited to ${MAX_LOGO_BYTES / 1024} KiB.`;
			return;
		}
		const reader = new FileReader();
		reader.onload = () => {
			pendingLogo = typeof reader.result === 'string' ? reader.result : null;
			removeLogo = false;
		};
		reader.onerror = () => (logoError = 'The file could not be read.');
		reader.readAsDataURL(file);
	}

	function dropLogo() {
		pendingLogo = null;
		removeLogo = true;
		logoError = null;
	}

	let homepageError = $state<string | null>(null);

	function toggleScene(id: string, on: boolean) {
		scenes = on ? [...scenes.filter((s) => s !== id), id] : scenes.filter((s) => s !== id);
	}

	// Picker state: per target, whether it is shown and how.
	interface Pick {
		label: string;
		group: string;
	}
	let picks = $state<Map<number, Pick>>(
		new Map((initial?.items ?? []).map((item) => [item.target_id, { label: item.label, group: item.group_name }]))
	);
	// Order of appearance = order of the original list, then order of ticking.
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

	function setPick(id: number, patch: Partial<Pick>) {
		const current = picks.get(id);
		if (!current) return;
		const next = new Map(picks);
		next.set(id, { ...current, ...patch });
		picks = next;
	}

	function move(id: number, delta: -1 | 1) {
		const index = order.indexOf(id);
		const target = index + delta;
		if (index < 0 || target < 0 || target >= order.length) return;
		const next = [...order];
		[next[index], next[target]] = [next[target], next[index]];
		order = next;
	}

	const groupsSeen = $derived([...new Set([...picks.values()].map((p) => p.group).filter(Boolean))]);
	const targetById = $derived(new Map(targets.map((t) => [t.id, t])));
	const sortedTargets = $derived([...targets].sort((a, b) => a.name.localeCompare(b.name)));

	let titleError = $state<string | null>(null);
	let slugError = $state<string | null>(null);
	let saving = $state(false);
	let error = $state<unknown>(null);

	function onTitleInput() {
		titleError = null;
		if (!slugTouched) slug = slugify(title);
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		titleError = title.trim() ? null : 'Give the page a title.';
		slugError = SLUG_RULE.test(slug) ? null : '2 to 40 characters: lowercase letters, digits and hyphens.';
		homepageError = homepageUrl.trim() === '' || /^https?:\/\/[^\s/]+/i.test(homepageUrl.trim()) ? null : 'Start with http:// or https://.';
		// The server has the full rule; this only catches the usual slips early.
		const host = domain.trim().replace(/^https?:\/\//i, '').replace(/\/$/, '');
		domainError =
			host === '' || /^[a-z0-9.-]+$/i.test(host) ? null : 'The host name only, like status.example.com: no path, port or space.';
		if (titleError || slugError || homepageError || domainError) return;

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
				footer_text: footerText.trim(),
				homepage_url: homepageUrl.trim(),
				subscribe_channel_id: subscribeChannel,
				domain: host === '' ? null : host
			};
			let saved = page ? await updateStatusPage(page.id, payload) : await createStatusPage(payload);
			if (pendingLogo) {
				await uploadStatusPageLogo(saved.id, pendingLogo);
				saved = { ...saved, logo_type: 'image' };
			} else if (removeLogo && hasStoredLogo) {
				await deleteStatusPageLogo(saved.id);
				saved = { ...saved, logo_type: null };
			}
			const items: StatusPageItemPayload[] = order
				.filter((id) => picks.has(id))
				.map((id) => {
					const pick = picks.get(id)!;
					return { target_id: id, label: pick.label.trim(), group_name: pick.group.trim() };
				});
			const savedItems = await setStatusPageItems(saved.id, items);
			onsaved({ ...saved, items: savedItems });
		} catch (cause) {
			// A refused domain (taken, invalid, DumbMonit's own address) belongs under its field.
			const api = toApiError(cause);
			if ((api.status === 400 || api.status === 409) && /domain|host name|IP address|punycode|DumbMonit itself/i.test(api.message)) {
				domainError = api.message;
			} else {
				error = cause;
			}
		} finally {
			saving = false;
		}
	}

	const idPrefix = $derived(page ? `sp-${page.id}` : 'sp-new');
</script>

<form class="grid gap-4" onsubmit={submit} novalidate>
	<div class="grid gap-4 sm:grid-cols-2">
		<Field label="Title" for="{idPrefix}-title" error={titleError} required>
			<input
				id="{idPrefix}-title"
				type="text"
				class="input"
				bind:value={title}
				oninput={onTitleInput}
				placeholder="Home lab"
				maxlength="120"
				disabled={saving}
				aria-invalid={titleError ? 'true' : undefined}
			/>
		</Field>
		<Field label="Address" for="{idPrefix}-slug" error={slugError} help={slug ? `Public URL: /s/${slug}` : 'Lowercase letters, digits and hyphens.'} required>
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
	</div>
	<Field label="Description" for="{idPrefix}-description" help="One sentence under the title. Optional.">
		<input id="{idPrefix}-description" type="text" class="input" bind:value={description} maxlength="1000" placeholder="What is up at home, at a glance." disabled={saving} />
	</Field>
	<div class="grid gap-4 sm:grid-cols-3">
		<Field label="Theme" for="{idPrefix}-theme">
			<select id="{idPrefix}-theme" class="input" bind:value={theme} disabled={saving}>
				<option value="auto">Follow the visitor's system</option>
				<option value="light">Day</option>
				<option value="dark">Night</option>
			</select>
		</Field>
		<Field label="History" for="{idPrefix}-days">
			<select id="{idPrefix}-days" class="input" bind:value={showDays} disabled={saving}>
				{#each HISTORY_CHOICES as choice (choice)}
					<option value={choice}>{choice} days</option>
				{/each}
			</select>
		</Field>
		<Field label="Published" for="{idPrefix}-published" inline help={published ? 'Anyone with the link can see it.' : 'Draft: answers 404 to visitors.'}>
			<Toggle id="{idPrefix}-published" bind:checked={published} disabled={saving} label="Published" />
		</Field>
	</div>

	<!-- Look -->
	<fieldset class="grid gap-4" disabled={saving}>
		<legend class="text-sm font-semibold text-ink">Look</legend>
		<div class="grid gap-4 sm:grid-cols-[auto_minmax(0,1fr)] sm:items-start">
			<div class="grid gap-2">
				<span class="text-sm text-ink" id="{idPrefix}-logo-label">Logo</span>
				<div class="flex items-center gap-3">
					<div class="flex size-16 items-center justify-center overflow-hidden rounded-lg border border-dashed border-line-strong bg-canvas-deep">
						{#if logoPreview}
							<img src={logoPreview} alt="Logo preview" class="size-full object-contain" />
						{:else}
							<span class="text-[0.6875rem] text-ink-3">None</span>
						{/if}
					</div>
					<div class="grid gap-1.5">
						<label class="inline-flex cursor-pointer items-center rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-semibold text-ink hover:bg-surface-2 focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-ink">
							{logoPreview ? 'Replace' : 'Upload'}
							<input type="file" accept="image/png,image/jpeg,image/webp" class="sr-only" onchange={pickLogo} aria-labelledby="{idPrefix}-logo-label" />
						</label>
						{#if logoPreview}
							<Button variant="ghost" size="sm" onclick={dropLogo}>Remove</Button>
						{/if}
					</div>
				</div>
				<p class="text-[0.8125rem] text-ink-2">PNG, JPEG or WebP, up to 256 KiB. Square works best.</p>
				{#if logoError}<p class="text-[0.8125rem] text-warning-ink" role="alert">{logoError}</p>{/if}
			</div>
			<div class="grid gap-4">
				<div class="grid gap-2">
					<span class="text-sm text-ink" id="{idPrefix}-accent-label">Accent</span>
					<div class="flex flex-wrap gap-2" role="radiogroup" aria-labelledby="{idPrefix}-accent-label">
						{#each ACCENTS as option (option.value)}
							<label class={`inline-flex cursor-pointer items-center gap-2 rounded-md border px-2.5 py-1.5 text-sm ${accent === option.value ? 'border-ink bg-surface-2 text-ink' : 'border-line text-ink-2 hover:text-ink'} ${accentClass(option.value)}`}>
								<input type="radio" class="sr-only" name="{idPrefix}-accent" value={option.value} bind:group={accent} />
								<span class="size-3.5 rounded-full bg-accent" aria-hidden="true"></span>
								{option.label}
							</label>
						{/each}
					</div>
					<p class="text-[0.8125rem] text-ink-2">Links, the top rule and the subscribe button. Each accent stays readable by day and by night.</p>
				</div>
				<Field label="Organisation website" for="{idPrefix}-homepage" error={homepageError} help="Optional link shown next to the title.">
					<input id="{idPrefix}-homepage" type="url" class="input" bind:value={homepageUrl} maxlength="300" placeholder="https://example.org" disabled={saving} oninput={() => (homepageError = null)} />
				</Field>
			</div>
		</div>
		<Field label="Footer text" for="{idPrefix}-footer" help="Plain text under the page, up to 280 characters: who runs it, how to reach them.">
			<textarea id="{idPrefix}-footer" class="input min-h-16" bind:value={footerText} maxlength="280" rows="2" disabled={saving}></textarea>
		</Field>
	</fieldset>

	<!-- Scene -->
	<fieldset class="grid gap-3" disabled={saving}>
		<legend class="text-sm font-semibold text-ink">Scene</legend>
		<p class="text-[0.8125rem] text-ink-2">
			An optional illustrated city behind the title of the public page. Off by default: tick none and the page stays plain. Tick several to alternate between them.
		</p>
		<SceneDefs />
		<div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
			{#each SCENE_CHOICES as choice (choice.value)}
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
							<span class="inline-flex min-w-5 items-center justify-center rounded-full bg-ink px-1.5 text-[0.6875rem] font-semibold text-canvas" aria-label={scenes.length > 1 ? `Position ${rank + 1}` : 'Selected'}>
								{scenes.length > 1 ? rank + 1 : '✓'}
							</span>
						{/if}
					</span>
				</label>
			{/each}
		</div>
		{#if scenes.length > 1}
			<Field label="Change scene" for="{idPrefix}-rotation" help="With several scenes ticked, which one a visitor sees and when it changes. They follow the order in which you ticked them.">
				<select id="{idPrefix}-rotation" class="input" bind:value={sceneRotation} disabled={saving}>
					{#each SCENE_ROTATIONS as option (option.value)}
						<option value={option.value}>{option.label}</option>
					{/each}
				</select>
			</Field>
		{/if}
	</fieldset>

	<!-- Email subscription -->
	<Field
		label="Email subscribers"
		for="{idPrefix}-subscribe"
		help={smtpChannels.length === 0
			? 'Add an email (SMTP) channel under Notifications to let visitors subscribe. Until then the page offers its RSS feed.'
			: 'Visitors confirm their address by email, and every update carries a one-click unsubscribe link.'}
	>
		<select
			id="{idPrefix}-subscribe"
			class="input"
			value={subscribeChannel === null ? '' : String(subscribeChannel)}
			onchange={(event) => (subscribeChannel = event.currentTarget.value === '' ? null : Number(event.currentTarget.value))}
			disabled={saving || (smtpChannels.length === 0 && subscribeChannel === null)}
		>
			<option value="">Off — RSS only</option>
			{#each smtpChannels as channel (channel.id)}
				<option value={String(channel.id)}>Send through “{channel.name}”{channel.enabled ? '' : ' (disabled)'}</option>
			{/each}
		</select>
	</Field>

	<!-- Public domain -->
	<DomainField
		id="{idPrefix}-domain"
		bind:value={domain}
		saved={initial?.domain ?? null}
		error={domainError}
		disabled={saving}
		oninput={() => (domainError = null)}
	/>

	<!-- Service picker -->
	<fieldset class="grid gap-2" disabled={saving}>
		<legend class="text-sm font-semibold text-ink">Services</legend>
		<p class="text-[0.8125rem] text-ink-2">Tick the devices to show. The label is what visitors read; a group heads a block of services.</p>
		{#if sortedTargets.length === 0}
			<p class="ghost-cell rounded-lg border border-dashed border-line px-4 py-6 text-center text-sm text-ink-2">No device yet. Add one on the Devices page first.</p>
		{:else}
			<ul class="divide-y divide-line rounded-[var(--radius-card)] border border-line" role="list">
				{#each sortedTargets as target (target.id)}
					{@const pick = picks.get(target.id)}
					<li class="px-3 py-2.5">
						<div class="flex items-center gap-3">
							<input
								id="{idPrefix}-pick-{target.id}"
								type="checkbox"
								class="size-4 shrink-0 accent-signal"
								checked={pick !== undefined}
								onchange={(event) => toggle(target, event.currentTarget.checked)}
							/>
							<label for="{idPrefix}-pick-{target.id}" class="min-w-0 flex-1 truncate text-sm text-ink">
								{target.name}
								<span class="text-ink-3">· {target.kind}</span>
							</label>
						</div>
						{#if pick}
							<div class="mt-2 grid gap-2 pl-7 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto] sm:items-center">
								<input
									type="text"
									class="input"
									value={pick.label}
									oninput={(event) => setPick(target.id, { label: event.currentTarget.value })}
									placeholder={target.name}
									maxlength="80"
									aria-label={`Public label of ${target.name}`}
								/>
								<input
									type="text"
									class="input"
									value={pick.group}
									oninput={(event) => setPick(target.id, { group: event.currentTarget.value })}
									placeholder="Group (optional)"
									maxlength="60"
									list="{idPrefix}-groups"
									aria-label={`Group of ${target.name}`}
								/>
								<div class="flex gap-1">
									<Button variant="ghost" size="sm" onclick={() => move(target.id, -1)} disabled={order.indexOf(target.id) <= 0}>Up</Button>
									<Button variant="ghost" size="sm" onclick={() => move(target.id, 1)} disabled={order.indexOf(target.id) >= order.length - 1}>Down</Button>
								</div>
							</div>
						{/if}
					</li>
				{/each}
			</ul>
			<datalist id="{idPrefix}-groups">
				{#each groupsSeen as group (group)}<option value={group}></option>{/each}
			</datalist>
			{#if order.length > 0}
				<p class="text-[0.8125rem] text-ink-2">
					Shown in this order: {order.map((id) => picks.get(id)?.label || targetById.get(id)?.name || id).join(', ')}.
				</p>
			{/if}
		{/if}
	</fieldset>

	{#if error}
		<ErrorNotice {error} title={page ? 'Could not save the page' : 'Could not create the page'} />
	{/if}

	<div class="flex flex-wrap items-center gap-2">
		<Button type="submit" variant="primary" loading={saving}>
			<Check class="size-4" aria-hidden="true" />
			{page ? 'Save changes' : 'Create page'}
		</Button>
		<Button variant="ghost" onclick={oncancel} disabled={saving}>
			<X class="size-4" aria-hidden="true" />
			Cancel
		</Button>
	</div>
</form>
