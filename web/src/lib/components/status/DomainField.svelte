<script lang="ts">
	/**
	 * "Public domain" of a status page: a host name (`status.example.com`) that a
	 * reverse proxy points at DumbMonit. On that name the server shows this page
	 * at `/` and nothing else. The "i" button opens the setup notes (click pins
	 * them, hover peeks); once saved, the resulting address is shown as a link.
	 */
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
		<label for={id} class="block text-sm font-semibold text-ink">Public domain</label>
		<button
			type="button"
			class="inline-flex size-6 items-center justify-center rounded-full text-ink-3 hover:bg-surface-2 hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink"
			aria-label="How a public domain works"
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
				<strong class="text-ink">What it does.</strong> Visitors who open this address see this status page
				straight away, at the root. Nothing else of DumbMonit answers there: no sign-in, no settings, no other
				page.
			</p>
			<ol class="grid list-decimal gap-1 pl-5">
				<li>Create the DNS record for the name (or the public hostname of a Cloudflare Tunnel), pointing at your reverse proxy.</li>
				<li>
					In the proxy, send the name to the address of DumbMonit, for example
					<code class="font-mono text-ink">http://dumbmonit:8080</code>, without rewriting the path.
				</li>
				<li>
					Pass the original <code class="font-mono text-ink">Host</code> header on. Nginx Proxy Manager, Traefik,
					Caddy and Cloudflare Tunnel do it by default; with plain Nginx add
					<code class="font-mono text-ink">proxy_set_header Host $host;</code>.
				</li>
			</ol>
			<p>
				HTTPS is handled by your proxy: links in emails and feeds use <code class="font-mono text-ink">https://</code>.
				If the domain shows the DumbMonit sign-in screen instead of the page, the proxy is not passing the Host
				header on; a bare "Not found" means it rewrites the path.
				<a class="font-semibold text-ink underline decoration-ink/30 underline-offset-2 hover:decoration-ink" href={DOCS_URL} target="_blank" rel="noreferrer">Examples for each proxy</a>
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
				Live at
				<a class="inline-flex items-center gap-1 font-mono font-semibold text-ink underline decoration-ink/30 underline-offset-2 hover:decoration-ink" href={`https://${saved}/`} target="_blank" rel="noreferrer">
					https://{saved}/
					<ExternalLink class="size-3" aria-hidden="true" />
				</a>
				once your proxy sends it here.
			</p>
		{:else}
			<p class="text-[0.8125rem] text-ink-2">Optional. Your own address for this page, served through your reverse proxy.</p>
		{/if}
	</div>
</div>
