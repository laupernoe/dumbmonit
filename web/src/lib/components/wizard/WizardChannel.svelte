<script lang="ts">
	/**
	 * Setup wizard, step "Get notified": the channel form the Notifications tab
	 * uses (built from `GET /api/notify/kinds`, so new kinds just appear), then
	 * a test message to prove it arrives.
	 */
	import { listChannelKinds, listChannels, testChannel, type Channel } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Button, ErrorNotice, Plate, Skeleton } from '#lib/ui/index.js';
	import ChannelForm from '#lib/components/notifications/ChannelForm.svelte';
	import { normalizeKind, type KindInfo } from '#lib/components/notifications/kinds.js';

	let kinds = $state<KindInfo[]>([]);
	let channels = $state<Channel[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let adding = $state(false);
	let testing = $state<number | null>(null);
	let result = $state<{ id: number; ok: boolean; message: string } | null>(null);

	$effect(() => {
		const controller = new AbortController();
		Promise.all([listChannelKinds(controller.signal), listChannels(controller.signal)])
			.then(([catalogue, list]) => {
				kinds = (Array.isArray(catalogue) ? catalogue : []).filter((k) => k && typeof k.kind === 'string').map(normalizeKind);
				channels = Array.isArray(list) ? list : [];
				adding = channels.length === 0;
			})
			.catch((cause) => {
				if (!(cause instanceof DOMException && cause.name === 'AbortError')) error = cause;
			})
			.finally(() => (loading = false));
		return () => controller.abort();
	});

	function saved(channel: Channel) {
		channels = [...channels.filter((c) => c.id !== channel.id), channel];
		adding = false;
		void test(channel);
	}

	async function test(channel: Channel) {
		testing = channel.id;
		result = null;
		try {
			const report = await testChannel(channel.id);
			result = { id: channel.id, ok: report.ok, message: report.message };
		} catch (cause) {
			result = { id: channel.id, ok: false, message: cause instanceof Error ? cause.message : String(cause) };
		} finally {
			testing = null;
		}
	}
</script>

{#if loading}
	<Skeleton class="h-48" />
{:else if error}
	<ErrorNotice {error} />
{:else}
	{#if channels.length > 0}
		<ul class="grid gap-2" aria-label={m.wizard_channel_list()}>
			{#each channels as channel (channel.id)}
				<li class="flex flex-wrap items-center gap-3 rounded-[var(--radius-card)] border border-line bg-surface px-4 py-3">
					<span class="min-w-0 flex-1">
						<span class="block truncate font-semibold text-ink">{channel.name}</span>
						<span class="block text-[0.8125rem] text-ink-2">{kinds.find((k) => k.kind === channel.kind)?.label ?? channel.kind}</span>
					</span>
					<Button size="sm" loading={testing === channel.id} disabled={testing !== null} onclick={() => test(channel)}>{m.wizard_channel_test()}</Button>
					{#if result?.id === channel.id}
						<div class="basis-full text-[0.8125rem]" aria-live="polite">
							{#if result.ok}
								<Plate tone="signal" draw>{m.wizard_channel_test_ok({ name: channel.name })}</Plate>
							{:else}
								<Plate tone="warning">{m.wizard_channel_test_failed({ error: result.message })}</Plate>
							{/if}
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}

	{#if adding}
		<div class="mt-4 rounded-[var(--radius-card)] border border-line bg-surface p-4">
			<ChannelForm {kinds} onsaved={saved} oncancel={() => (adding = false)} />
		</div>
	{:else}
		<Button class="mt-4" onclick={() => (adding = true)}>{m.wizard_channel_add()}</Button>
	{/if}
{/if}
