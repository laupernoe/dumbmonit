<script lang="ts">
	/**
	 * Sentinel mode of an agent device (administrators): the agent probes the
	 * addresses from where it stands and notifies by itself, without going
	 * through the server, when they stop answering. The notification channel is a
	 * secret: it is sent here, stored encrypted, and never shown again.
	 */
	import { getAgentSentinel, setAgentSentinel } from '#lib/api/agent_sentinel.js';
	import type { AgentSentinel, AgentSentinelInput, SentinelChannel, Target } from '#lib/api/types.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, ErrorNotice, Field, Panel, Plate, Toggle } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	type Kind = 'webhook' | 'ntfy';

	let current = $state<AgentSentinel | null>(null);
	let enabled = $state(false);
	let urls = $state('');
	let interval = $state(60);
	let threshold = $state(3);
	let kind = $state<Kind>('webhook');
	let webhookUrl = $state('');
	let ntfyServer = $state('https://ntfy.sh');
	let ntfyTopic = $state('');
	let clear = $state(false);
	let saving = $state(false);
	let saved = $state(false);
	let error = $state<unknown>(null);

	/** One address per line, followed by its certificate fingerprint when it has one. */
	const asLines = (value: AgentSentinel) =>
		value.urls.map((url) => (value.pins[url] ? `${url} ${value.pins[url]}` : url)).join('\n');

	function adopt(value: AgentSentinel) {
		current = value;
		enabled = value.enabled;
		urls = asLines(value);
		interval = value.interval_secs;
		threshold = value.threshold;
		kind = value.channel_kind ?? 'webhook';
		webhookUrl = '';
		ntfyTopic = '';
		clear = false;
	}

	const lines = (text: string) =>
		text
			.split('\n')
			.map((line) => line.trim())
			.filter(Boolean);

	$effect(() => {
		if (!auth.isAdmin) return;
		const controller = new AbortController();
		getAgentSentinel(target.id, controller.signal)
			.then(adopt)
			.catch(() => (current = null));
		return () => controller.abort();
	});

	/** The channel typed in the form; nothing typed means "keep the stored one". */
	function typedChannel(): SentinelChannel | undefined {
		if (kind === 'webhook') {
			return webhookUrl.trim() ? { kind: 'webhook', url: webhookUrl.trim() } : undefined;
		}
		return ntfyTopic.trim() ? { kind: 'ntfy', server: ntfyServer.trim(), topic: ntfyTopic.trim() } : undefined;
	}

	/** Splits the lines into addresses and their optional fingerprints. */
	function parsed(): { urls: string[]; pins: Record<string, string> } {
		const list: string[] = [];
		const pins: Record<string, string> = {};
		for (const line of lines(urls)) {
			const [url, ...rest] = line.split(/\s+/);
			list.push(url);
			if (rest.length) pins[url] = rest.join('');
		}
		return { urls: list, pins };
	}

	const dirty = $derived(
		current !== null &&
			(enabled !== current.enabled ||
				urls !== asLines(current) ||
				interval !== current.interval_secs ||
				threshold !== current.threshold ||
				clear ||
				typedChannel() !== undefined)
	);

	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (saving) return;
		saving = true;
		saved = false;
		error = null;
		try {
			const { urls: list, pins } = parsed();
			const body: AgentSentinelInput = {
				enabled,
				urls: list,
				pins,
				interval_secs: Number(interval),
				threshold: Number(threshold)
			};
			if (clear) body.clear_channel = true;
			else {
				const channel = typedChannel();
				if (channel) body.channel = channel;
			}
			adopt(await setAgentSentinel(target.id, body));
			saved = true;
		} catch (cause) {
			error = cause;
		} finally {
			saving = false;
		}
	}
</script>

{#if current && auth.isAdmin}
	<Panel title={m.agentsentinel_title()}>
		{#snippet aside()}
			<Plate tone={current?.enabled ? 'signal' : 'ghost'} label={current?.enabled ? m.agentsentinel_on() : m.agentsentinel_off()} />
		{/snippet}

		{#if !current.encrypted}
			<p class="mb-4 rounded-md border border-warning-ink/40 p-3 text-sm text-warning-ink" role="alert">{m.agentsentinel_not_tls()}</p>
		{/if}

		<form class="grid gap-4" onsubmit={save}>
			<div class="flex items-start gap-3">
				<Toggle id="agent-sentinel-switch" bind:checked={enabled} label={m.agentsentinel_switch()} disabled={saving} />
				<div class="min-w-0">
					<label for="agent-sentinel-switch" class="text-sm font-semibold text-ink">{m.agentsentinel_switch()}</label>
					<p class="text-sm text-ink-2">{m.agentsentinel_switch_help()}</p>
				</div>
			</div>

			{#if enabled}
				<Field label={m.agentsentinel_urls()} for="agent-sentinel-urls" help={m.agentsentinel_urls_help()}>
					<textarea
						id="agent-sentinel-urls"
						class="input min-h-20 font-mono text-[0.8125rem]"
						rows="3"
						spellcheck="false"
						autocomplete="off"
						placeholder="https://dumbmonit.example.org:8443/api/health\nhttps://gateway.example.org/  3f2a…(SHA-256)"
						disabled={saving}
						bind:value={urls}
					></textarea>
				</Field>
				<p class="text-sm text-ink-2">{m.agentsentinel_pin_help()}</p>
				<div class="grid gap-4 sm:grid-cols-2">
					<Field label={m.agentsentinel_interval()} for="agent-sentinel-interval">
						<input id="agent-sentinel-interval" class="input tnum" type="number" min="15" max="3600" bind:value={interval} disabled={saving} />
					</Field>
					<Field label={m.agentsentinel_threshold()} for="agent-sentinel-threshold" help={m.agentsentinel_threshold_help()}>
						<input id="agent-sentinel-threshold" class="input tnum" type="number" min="1" max="20" bind:value={threshold} disabled={saving} />
					</Field>
				</div>

				<Field label={m.agentsentinel_channel()} for="agent-sentinel-kind" help={m.agentsentinel_channel_help()}>
					<select id="agent-sentinel-kind" class="input" bind:value={kind} disabled={saving || clear}>
						<option value="webhook">{m.agentsentinel_kind_webhook()}</option>
						<option value="ntfy">{m.agentsentinel_kind_ntfy()}</option>
					</select>
				</Field>
				{#if kind === 'webhook'}
					<Field label={m.agentsentinel_webhook()} for="agent-sentinel-webhook">
						<input
							id="agent-sentinel-webhook"
							class="input font-mono text-[0.8125rem]"
							type="password"
							autocomplete="off"
							spellcheck="false"
							placeholder={current.has_secret && current.channel_kind === 'webhook' ? m.agentsentinel_keep() : 'https://'}
							disabled={saving || clear}
							bind:value={webhookUrl}
						/>
					</Field>
				{:else}
					<div class="grid gap-4 sm:grid-cols-2">
						<Field label={m.agentsentinel_ntfy_server()} for="agent-sentinel-ntfy-server">
							<input id="agent-sentinel-ntfy-server" class="input font-mono text-[0.8125rem]" type="url" autocomplete="off" disabled={saving || clear} bind:value={ntfyServer} />
						</Field>
						<Field label={m.agentsentinel_ntfy_topic()} for="agent-sentinel-ntfy-topic">
							<input
								id="agent-sentinel-ntfy-topic"
								class="input font-mono text-[0.8125rem]"
								type="password"
								autocomplete="off"
								spellcheck="false"
								placeholder={current.has_secret && current.channel_kind === 'ntfy' ? m.agentsentinel_keep() : ''}
								disabled={saving || clear}
								bind:value={ntfyTopic}
							/>
						</Field>
					</div>
				{/if}
				{#if current.has_secret}
					<label class="inline-flex min-h-10 items-center gap-2 text-sm text-ink">
						<input type="checkbox" class="size-4 accent-[var(--c-signal)]" bind:checked={clear} disabled={saving} />
						{m.agentsentinel_clear({ kind: current.channel_kind ?? '' })}
					</label>
				{/if}
			{/if}

			<p class="text-sm text-ink-2">{m.agentsentinel_note()}</p>

			{#if saved && !dirty}
				<p class="text-sm text-ink-2" role="status">{m.agentsentinel_pending()}</p>
			{/if}
			{#if error}
				<ErrorNotice {error} title={m.agentsentinel_save_error()} />
			{/if}
			<div>
				<Button type="submit" variant="secondary" size="sm" loading={saving} success={saved} disabled={!dirty}>{m.agentsentinel_save()}</Button>
			</div>
		</form>
	</Panel>
{/if}
