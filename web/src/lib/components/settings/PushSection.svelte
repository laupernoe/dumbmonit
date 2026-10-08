<script lang="ts">
	/**
	 * Settings → Push notifications: native notifications on this phone or
	 * computer through Web Push, with no app to install. Per account: each
	 * person enables their own devices (viewers too), and a "Web Push" channel
	 * in Alerts → Notifications decides which alerts reach them.
	 *
	 * The browser side (permission, service worker, subscription) lives in
	 * `$lib/push`; the server side in `$lib/api/webpush`.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { BellRing, Info, Send, Smartphone } from 'lucide-svelte';
	import type { PushDevice, PushOverview } from '#lib/api/index.js';
	import { deletePushDevice, getPushOverview, savePushSubscription, sendPushTest } from '#lib/api/webpush.js';
	import { currentSubscription, fingerprint, isStandalone, pushSupport, subscribe, type PushSupport } from '#lib/push.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { Button, Confirm, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';

	let overview = $state<PushOverview | null>(null);
	let loadError = $state<unknown>(null);
	let support = $state<PushSupport>({ state: 'unsupported' });
	/** Fingerprint of this browser's subscription, when it has one. */
	let here = $state<string | null>(null);
	let busy = $state<'enable' | 'test' | number | null>(null);
	let actionError = $state<unknown>(null);
	let notice = $state<{ ok: boolean; text: string } | null>(null);
	let helpOpen = $state(false);

	const thisDevice = $derived(overview?.devices.find((d) => d.fingerprint === here) ?? null);
	const origin = typeof window === 'undefined' ? '' : window.location.origin;

	async function load() {
		loadError = null;
		support = pushSupport();
		try {
			overview = await getPushOverview();
		} catch (cause) {
			loadError = cause;
			return;
		}
		if (support.state !== 'ready') return;
		try {
			const subscription = await currentSubscription();
			here = subscription ? await fingerprint(subscription.endpoint) : null;
			// The browser may have renewed its subscription on its own, or the
			// server lost it (restore, removal from another device): send it again
			// so alerts keep coming. Same endpoint → same row, no duplicate.
			if (subscription && Notification.permission === 'granted' && !overview.devices.some((d) => d.fingerprint === here)) {
				const saved = await savePushSubscription(subscription.toJSON());
				overview = { ...overview, devices: [saved, ...overview.devices] };
			}
		} catch {
			/* Reading the local subscription is best effort. */
		}
	}

	$effect(() => {
		void load();
	});

	async function enable() {
		if (!overview) return;
		busy = 'enable';
		actionError = null;
		notice = null;
		try {
			const subscription = await subscribe(overview.public_key);
			const saved = await savePushSubscription(subscription.toJSON());
			here = saved.fingerprint;
			overview = { ...overview, devices: [saved, ...overview.devices.filter((d) => d.id !== saved.id)] };
			notice = { ok: true, text: m.settings_push_enabled_notice() };
		} catch (cause) {
			actionError = cause;
		} finally {
			support = pushSupport();
			busy = null;
		}
	}

	async function test() {
		busy = 'test';
		actionError = null;
		notice = null;
		try {
			const report = await sendPushTest();
			notice = { ok: report.ok, text: report.message };
			overview = await getPushOverview();
		} catch (cause) {
			actionError = cause;
		} finally {
			busy = null;
		}
	}

	async function remove(device: PushDevice) {
		busy = device.id;
		actionError = null;
		notice = null;
		try {
			// Removing this very browser also drops its local subscription, so
			// the push service forgets it too.
			if (device.fingerprint === here) {
				const subscription = await currentSubscription().catch(() => null);
				await subscription?.unsubscribe().catch(() => false);
				here = null;
			}
			await deletePushDevice(device.id);
			if (overview) overview = { ...overview, devices: overview.devices.filter((d) => d.id !== device.id) };
		} catch (cause) {
			actionError = cause;
		} finally {
			busy = null;
		}
	}
</script>

<Panel id="push" title={m.settings_push_title()} description={m.settings_push_description()}>
	{#snippet aside()}
		<button
			type="button"
			class="inline-flex size-8 items-center justify-center rounded-full border border-line text-ink-2 transition-colors hover:bg-surface-2 hover:text-ink"
			aria-expanded={helpOpen}
			aria-controls="push-help"
			aria-label={m.settings_push_help_label()}
			title={m.settings_push_help_label()}
			onclick={() => (helpOpen = !helpOpen)}
		>
			<Info class="size-4" aria-hidden="true" />
		</button>
	{/snippet}

	{#if helpOpen}
		<div id="push-help" class="mb-4 rounded-lg border border-line bg-canvas-deep px-4 py-3 text-[0.8125rem] leading-relaxed text-ink-2">
			<ul class="list-disc space-y-1 pl-4">
				<li><span class="font-semibold text-ink">{m.settings_push_help_https_lead()}</span> {m.settings_push_help_https_body()}</li>
				<li><span class="font-semibold text-ink">{m.settings_push_help_ios_lead()}</span> {m.settings_push_help_ios_body()}</li>
				<li><span class="font-semibold text-ink">{m.settings_push_help_desktop_lead()}</span> {m.settings_push_help_desktop_body()}</li>
				<li><span class="font-semibold text-ink">{m.settings_push_help_domains_lead()}</span> {m.settings_push_help_domains_body()}</li>
				<li><a href="/alerts#notifications" class="font-semibold text-ink hover:underline">{m.settings_push_help_channel()}</a></li>
			</ul>
			<a href="https://dumbmonit.readthedocs.io/en/latest/using/push-notifications/" target="_blank" rel="noopener" class="mt-2 inline-block font-semibold text-signal-ink hover:underline">{m.settings_push_help_guide()}</a>
		</div>
	{/if}

	{#if loadError}
		<ErrorNotice error={loadError} title={m.settings_push_load_error()} onretry={() => void load()} />
	{:else if !overview}
		<Skeleton class="h-10 w-full" rows={2} />
	{:else}
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div class="min-w-0">
				<p class="text-sm font-semibold text-ink">{m.settings_push_this_device()}</p>
				<div class="mt-1 flex flex-wrap items-center gap-2 text-[0.8125rem] text-ink-2">
					{#if support.state === 'insecure'}
						<Plate tone="advisory" label={m.settings_push_plate_https()} />
						<span>{m.settings_push_text_https({ origin })}</span>
					{:else if support.state === 'ios-install'}
						<Plate tone="advisory" label={m.settings_push_plate_install()} />
						<span>{m.settings_push_text_install()}</span>
					{:else if support.state === 'unsupported'}
						<Plate tone="muted" label={m.settings_push_plate_unsupported()} />
						<span>{m.settings_push_text_unsupported()}</span>
					{:else if support.permission === 'denied'}
						<Plate tone="warning" label={m.settings_push_plate_blocked()} />
						<span>{m.settings_push_text_blocked()}</span>
					{:else if thisDevice}
						<Plate tone="signal" label={m.settings_push_plate_enabled()} />
						<span>{isStandalone() ? m.settings_push_device_installed({ device: thisDevice.device }) : thisDevice.device}</span>
					{:else}
						<Plate tone="ghost" label={m.settings_push_plate_off()} />
						<span>{m.settings_push_text_off()}</span>
					{/if}
				</div>
			</div>
			<div class="flex flex-wrap items-center gap-2">
				{#if support.state === 'ready' && support.permission !== 'denied' && !thisDevice}
					<Button size="sm" variant="primary" loading={busy === 'enable'} disabled={busy !== null} onclick={() => void enable()}>
						<BellRing class="size-4" aria-hidden="true" />
						{m.settings_push_enable()}
					</Button>
				{/if}
				{#if overview.devices.length > 0}
					<Button size="sm" variant="secondary" loading={busy === 'test'} disabled={busy !== null} onclick={() => void test()}>
						<Send class="size-4" aria-hidden="true" />
						{m.settings_push_test()}
					</Button>
				{/if}
			</div>
		</div>

		{#if notice}
			<p class={`mt-3 text-[0.8125rem] ${notice.ok ? 'text-signal-ink' : 'text-warning-ink'}`} role="status">{notice.text}</p>
		{/if}
		{#if actionError}
			<div class="mt-3"><ErrorNotice error={actionError} title={m.settings_push_action_error()} /></div>
		{/if}

		<div class="mt-5 border-t border-line pt-4">
			<p class="text-sm font-semibold text-ink">{m.settings_push_subscribed()}</p>
			{#if overview.devices.length === 0}
				<p class="mt-1 text-[0.8125rem] text-ink-2">{m.settings_push_none()}</p>
			{:else}
				<ul class="mt-2 divide-y divide-line rounded-lg border border-line">
					{#each overview.devices as device (device.id)}
						<li class="flex flex-wrap items-center justify-between gap-3 px-3 py-2.5">
							<div class="flex min-w-0 items-start gap-2.5">
								<Smartphone class="mt-0.5 size-4 shrink-0 text-ink-3" aria-hidden="true" />
								<div class="min-w-0">
									<p class="text-sm text-ink">
										{device.device || m.settings_push_browser()}
										{#if device.fingerprint === here}<span class="ml-1 text-[0.75rem] font-semibold text-signal-ink">{m.settings_push_this_device_tag()}</span>{/if}
									</p>
									<p class="text-[0.75rem] text-ink-2">
										{device.last_success_at
											? m.settings_push_meta_delivered({ service: device.push_service, date: formatDateTime(device.created_at), when: formatRelative(device.last_success_at) })
											: m.settings_push_meta_added({ service: device.push_service, date: formatDateTime(device.created_at) })}
									</p>
									{#if device.last_error}
										<p class="mt-0.5 text-[0.75rem] break-words text-warning-ink">{m.settings_push_last_failed({ error: device.last_error })}</p>
									{/if}
								</div>
							</div>
							<Confirm confirmLabel={m.settings_push_remove_confirm()} loading={busy === device.id} disabled={busy !== null && busy !== device.id} onconfirm={() => remove(device)}>
								{m.settings_push_remove()}
							</Confirm>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}
</Panel>
