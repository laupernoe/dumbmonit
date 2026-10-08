<script lang="ts">
	/**
	 * "Public domain" of a status page: a host name (`status.example.com`) that a
	 * reverse proxy points at DumbMonit. On that name the server shows this page
	 * at `/` and nothing else. The "i" button opens the setup notes (click pins
	 * them, hover peeks); once saved, the resulting address is shown as a link.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { ExternalLink, Info } from 'lucide-svelte';

	const DOCS_URL = 'https://dumbmonit.readthedocs.io/en/latest/using/status-pages/#custom-domain';

	interface Props {
		id: string;
		value: string;
		/** The domain as stored, for the link under the field; `null` before saving. */
		saved: string | null;
		error?: string | null;
		disabled?: boolean;
		oninput?: () => void;
	}

	let { id, value = $bindable(), saved, error = null, disabled = false, oninput }: Props = $props();

	let pinned = $state(false);
	let hovering = $state(false);
	const open = $derived(pinned || hovering);

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && open) {
			pinned = false;
			hovering = false;
		}
	}
</script>

<div class="grid gap-1.5">
	<div class="flex items-center gap-1.5">
		<label for={id} class="block text-sm font-semibold text-ink">{m.status_domain_label()}</label>
		<button
			type="button"
			class="inline-flex size-6 items-center justify-center rounded-full text-ink-3 hover:bg-surface-2 hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
			aria-label={m.status_domain_info_aria()}
			aria-expanded={open}
			aria-controls="{id}-info"
			onclick={() => (pinned = !pinned)}
			onmouseenter={() => (hovering = true)}
			onmouseleave={() => (hovering = false)}
			{onkeydown}
		>
			<Info class="size-4" aria-hidden="true" />
		</button>
	</div>
	{#if open}
		<div
			id="{id}-info"
			class="grid gap-2 rounded-lg border border-line bg-surface-2 p-3 text-[0.8125rem] text-ink-2"
			role="note"
		>
			<p>
				<strong class="text-ink">{m.status_domain_what_title()}</strong> {m.status_domain_what_body()}
			</p>
			<ol class="grid list-decimal gap-1 pl-5">
				<li>{m.status_domain_step_dns()}</li>
				<li>
					{m.status_domain_step_proxy({ url: 'http://dumbmonit:8080' })}
				</li>
				<li>
					{m.status_domain_step_host({ header: 'Host', directive: 'proxy_set_header Host $host;' })}
				</li>
			</ol>
			<p>
				{m.status_domain_https({ scheme: 'https://' })}
				<a class="font-semibold text-ink underline decoration-ink/30 underline-offset-2 hover:decoration-ink" href={DOCS_URL} target="_blank" rel="noreferrer">{m.status_domain_examples()}</a>
			</p>
		</div>
	{/if}
	<input
		{id}
		type="text"
		class="input font-mono"
		bind:value
		{oninput}
		placeholder="status.example.com"
		maxlength="253"
		autocomplete="off"
		autocapitalize="off"
		spellcheck="false"
		inputmode="url"
		{disabled}
		aria-invalid={error ? 'true' : undefined}
		aria-describedby="{id}-help"
	/>
	<div id="{id}-help">
		{#if error}
			<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">{error}</p>
		{:else if saved}
			<p class="text-[0.8125rem] text-ink-2">
				{m.status_domain_live_at()}
				<a class="inline-flex items-center gap-1 font-mono font-semibold text-ink underline decoration-ink/30 underline-offset-2 hover:decoration-ink" href={`https://${saved}/`} target="_blank" rel="noreferrer">
					https://{saved}/
					<ExternalLink class="size-3" aria-hidden="true" />
				</a>
				{m.status_domain_live_when()}
			</p>
		{:else}
			<p class="text-[0.8125rem] text-ink-2">{m.status_domain_help()}</p>
		{/if}
	</div>
</div>
