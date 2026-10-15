<script lang="ts">
	/**
	 * Settings → the security score switch (off by default). On, rated devices
	 * get a grade badge in the device list and on their page, opening a report
	 * of what to improve. Self-contained (own load, own save, own errors) so it
	 * can sit in any settings section; `/security` redirects here
	 * (`#security-score`). Only administrators can flip it.
	 */
	import { ExternalLink } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getSecuritySettings, setSecuritySettings } from '#lib/api/security.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { ErrorNotice, Plate, Toggle } from '#lib/ui/index.js';

	const DOCS_URL = 'https://dumbmonit.readthedocs.io/en/latest/using/security-score/';

	/** `null` until the server answered. */
	let saved = $state<boolean | null>(null);
	/** The switch's own state, put back to the server's when a save fails. */
	let enabled = $state(false);
	let saving = $state(false);
	let error = $state<unknown>(null);

	$effect(() => {
		const controller = new AbortController();
		getSecuritySettings(controller.signal)
			.then((value) => {
				saved = value.enabled;
				enabled = value.enabled;
			})
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				error = cause;
			});
		return () => controller.abort();
	});

	async function toggle(value: boolean) {
		error = null;
		saving = true;
		try {
			saved = (await setSecuritySettings(value)).enabled;
			enabled = saved;
		} catch (cause) {
			error = cause;
			enabled = saved ?? false;
		} finally {
			saving = false;
		}
	}
</script>

<section id="security-score" class="mt-6 border-t border-line pt-5" aria-labelledby="security-score-title">
	<div class="flex items-start justify-between gap-4">
		<div class="min-w-0">
			<h3 id="security-score-title" class="text-sm font-semibold text-ink">{m.settings_security_score_title()}</h3>
			<p class="mt-0.5 max-w-prose text-[0.8125rem] text-ink-2">{m.settings_security_score_hint()}</p>
			<p class="mt-1 max-w-prose text-[0.8125rem] text-ink-2">{m.settings_security_score_disclaimer()}</p>
			<a
				href={DOCS_URL}
				target="_blank"
				rel="noopener"
				class="mt-1 inline-flex min-h-10 items-center gap-1 text-[0.8125rem] font-semibold text-ink-2 hover:text-ink hover:underline"
			>
				{m.settings_security_score_docs()}
				<ExternalLink class="size-3.5" aria-hidden="true" />
			</a>
		</div>
		<div class="flex min-h-10 shrink-0 items-center gap-2">
			{#if saved !== null}
				<Plate tone={enabled ? 'signal' : 'ghost'} bare label={enabled ? m.settings_security_score_on() : m.settings_security_score_off()} />
			{/if}
			<Toggle
				id="security-score-toggle"
				bind:checked={enabled}
				disabled={saved === null || saving || !auth.isAdmin}
				label={m.settings_security_score_toggle()}
				onchange={(value) => void toggle(value)}
			/>
		</div>
	</div>
	{#if saved !== null && !auth.isAdmin}
		<p class="mt-2 text-[0.8125rem] text-ink-3">{m.settings_security_score_admin_only()}</p>
	{/if}
	{#if error}
		<ErrorNotice {error} title={m.settings_security_score_error()} class="mt-3" />
	{/if}
</section>
