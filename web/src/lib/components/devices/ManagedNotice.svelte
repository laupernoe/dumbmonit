<script lang="ts">
	/**
	 * Shown on a monitor that Docker labels manage: it is read-only here, and the
	 * notice says why and how to take it over ("Release"). An archived monitor
	 * (no label declares it any more) says so. The caller decides who sees the
	 * release button (administrators).
	 */
	import { Lock } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { releaseManagedTarget } from '#lib/api/index.js';
	import type { Target } from '#lib/api/types.js';
	import { formatDateTime } from '#lib/format.js';
	import { Button, ErrorNotice, Plate } from '#lib/ui/index.js';

	interface Props {
		target: Target;
		/** Offer the release button. */
		canRelease?: boolean;
		/** Called after a successful release, so the page reloads the monitor. */
		onreleased?: () => void;
		class?: string;
	}
	let { target, canRelease = false, onreleased, class: className = '' }: Props = $props();

	let busy = $state(false);
	let error = $state<unknown>(null);

	async function release() {
		busy = true;
		error = null;
		try {
			await releaseManagedTarget(target.id);
			onreleased?.();
		} catch (cause) {
			error = cause;
		} finally {
			busy = false;
		}
	}
</script>

{#if target.managed}
	<div class="rounded-lg border border-line bg-surface-2 px-4 py-3 {className}">
		<div class="flex flex-wrap items-center gap-2">
			<Plate tone="info">
				<Lock class="size-3.5" aria-hidden="true" />
				{m.devices_managed_badge()}
			</Plate>
			{#if target.managed.missing_since}
				<Plate tone="advisory" label={m.devices_managed_archived({ date: formatDateTime(target.managed.missing_since) })} />
			{/if}
		</div>
		<p class="mt-2 text-sm text-ink-2">{m.devices_managed_notice({ key: target.managed.key })}</p>
		{#if canRelease}
			<div class="mt-3 flex flex-wrap items-center gap-3">
				<Button variant="secondary" loading={busy} onclick={release}>{m.devices_managed_release()}</Button>
				<span class="text-sm text-ink-3">{m.devices_managed_release_hint()}</span>
			</div>
		{/if}
		{#if error}<ErrorNotice {error} title={m.devices_managed_release_error()} class="mt-3" />{/if}
	</div>
{/if}
