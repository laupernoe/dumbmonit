<script lang="ts">
	import { withBase } from '#lib/paths.js';
	/**
	 * Strip shown while this session travels in clear: the page is on plain
	 * HTTP (neither HTTPS to this server nor through an HTTPS reverse proxy),
	 * outside the loopback. Passwords, session cookie and data can be read by
	 * anyone on the path.
	 *
	 * When the server also serves HTTPS on this port, it offers the same page
	 * over HTTPS, with the certificate fingerprint to compare with the
	 * browser's warning (self-signed). Otherwise it says how to get HTTPS.
	 * Dismissed for the browser session only: it comes back next time.
	 */
	import { LockOpen } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { auth } from '#lib/stores/auth.svelte.js';

	const DISMISSED = 'dumbmonit.cleartext-banner.dismissed';
	const DOC = 'https://dumbmonit.selfhostr.io/docs/install/https/';

	function loopback(host: string): boolean {
		return host === 'localhost' || host === '[::1]' || host.startsWith('127.');
	}

	let dismissed = $state(false);
	let plain = $state(false);
	let target = $state('');
	$effect(() => {
		plain = window.location.protocol === 'http:' && !loopback(window.location.hostname);
		target = `https://${window.location.host}${window.location.pathname}${window.location.search}`;
		try {
			dismissed = sessionStorage.getItem(DISMISSED) === '1';
		} catch {
			dismissed = false;
		}
	});

	function dismiss() {
		dismissed = true;
		try {
			sessionStorage.setItem(DISMISSED, '1');
		} catch {
			// Private mode: dismissed for this page only.
		}
	}

	const https = $derived(auth.https);
	const visible = $derived(plain && !dismissed && !auth.demo);
	const link = 'inline-flex min-h-10 items-center rounded-md px-2 font-semibold text-warning-ink underline-offset-2 hover:underline';
</script>

{#if visible}
	<div role="alert" class="border-b border-warning/50 bg-warning-soft text-[0.8125rem]">
		<div class="mx-auto flex max-w-7xl flex-wrap items-center gap-x-3 gap-y-1 px-4 py-1.5 sm:px-6">
			<p class="flex min-w-0 flex-1 items-start gap-2 text-ink">
				<LockOpen class="mt-0.5 size-4 shrink-0 text-warning-ink" aria-hidden="true" />
				<span class="min-w-0">
					<span class="font-semibold">{m.app_cleartext_title()}</span>
					{' '}{https?.available ? m.app_cleartext_body_available() : m.app_cleartext_body_unavailable()}
					{#if https?.available && https.self_signed && https.fingerprint_sha256}
						<span class="mt-0.5 block font-mono text-[0.75rem] break-all text-ink-2">{m.app_cleartext_fingerprint({ fingerprint: https.fingerprint_sha256 })}</span>
					{/if}
				</span>
			</p>
			<div class="flex flex-wrap items-center gap-1">
				{#if https?.available}
					<a class={link} href={withBase(target)}>{m.app_cleartext_switch()}</a>
				{:else}
					<a class={link} href={withBase(DOC)} target="_blank" rel="noopener">{m.app_cleartext_how()}</a>
				{/if}
				<button type="button" class="inline-flex min-h-10 items-center rounded-md px-2 text-ink-2 hover:bg-surface-2 hover:text-ink" onclick={dismiss}>
					{m.app_cleartext_dismiss()}
				</button>
			</div>
		</div>
	</div>
{/if}
