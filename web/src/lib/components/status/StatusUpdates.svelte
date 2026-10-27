<script lang="ts">
	/**
	 * "Get updates" on a public page: the RSS feed always, and subscription by
	 * email, SMS, signed webhook or Slack when the owner opened them. The answer
	 * is the same whether the destination was new, pending or already subscribed:
	 * the page tells no one who follows it.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { CalendarDays, Check, Mail, Rss } from 'lucide-svelte';
	import { confirmStatusSmsCode, subscribeToStatus, toApiError, type StatusSubscribePayload, type StatusSubscriptionKind } from '#lib/api/index.js';

	interface Props {
		slug: string;
		/** Subscriptions the page offers. */
		kinds: StatusSubscriptionKind[];
		/** Services a subscriber may pick (public key and name). */
		services?: { key: string; label: string }[];
	}

	let { slug, kinds, services = [] }: Props = $props();

	const subscribe = $derived(kinds.length > 0);
	const LABEL: Record<StatusSubscriptionKind, () => string> = {
		email: () => m.status_sub_kind_email(),
		sms: () => m.status_sub_kind_sms(),
		webhook: () => m.status_sub_kind_webhook(),
		slack: () => m.status_sub_kind_slack()
	};

	let chosen = $state<StatusSubscriptionKind | null>(null);
	const kind = $derived(chosen && kinds.includes(chosen) ? chosen : (kinds[0] ?? 'email'));
	let email = $state('');
	let phone = $state('');
	let url = $state('');
	let picked = $state<string[]>([]);
	let code = $state('');
	let sending = $state(false);
	let step = $state<'form' | 'code' | 'done'>('form');
	let done = $state<string | null>(null);
	let secret = $state<string | null>(null);
	let error = $state<string | null>(null);

	const rss = $derived(`/api/public/status/${encodeURIComponent(slug)}/rss`);
	const ical = $derived(`/api/public/status/${encodeURIComponent(slug)}/maintenance.ics`);

	function payload(): StatusSubscribePayload | null {
		const components = picked.length > 0 ? picked : undefined;
		if (kind === 'email') {
			if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.trim())) {
				error = m.status_updates_invalid_email();
				return null;
			}
			return { kind, email: email.trim(), components };
		}
		if (kind === 'sms') {
			if (!/^\+[1-9][\d\s().-]{7,20}$/.test(phone.trim())) {
				error = m.status_sub_phone_invalid();
				return null;
			}
			return { kind, phone: phone.trim(), components };
		}
		if (kind === 'slack') {
			if (!/^https:\/\/hooks\.slack(-gov)?\.com\/(services|workflows)\//.test(url.trim())) {
				error = m.status_sub_slack_invalid();
				return null;
			}
			return { kind, url: url.trim(), components };
		}
		if (!/^https?:\/\/[^\s/]+/.test(url.trim())) {
			error = m.status_sub_url_invalid();
			return null;
		}
		return { kind, url: url.trim(), components };
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		const body = payload();
		if (!body) return;
		sending = true;
		try {
			const reply = await subscribeToStatus(slug, body);
			done = reply.message;
			secret = reply.secret ?? null;
			step = reply.confirm === 'code' ? 'code' : 'done';
			email = '';
			url = '';
		} catch (cause) {
			const apiError = toApiError(cause);
			error = apiError.status === 429 ? m.status_updates_rate_limited() : apiError.message;
		} finally {
			sending = false;
		}
	}

	async function confirmCode(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		sending = true;
		try {
			await confirmStatusSmsCode(slug, phone.trim(), code.trim());
			done = m.status_sub_code_done();
			step = 'done';
		} catch (cause) {
			const apiError = toApiError(cause);
			error = apiError.status === 429 ? m.status_updates_rate_limited() : apiError.status === 400 ? m.status_sub_code_invalid() : apiError.message;
		} finally {
			sending = false;
		}
	}

	const inputClass = 'input min-w-0 flex-1';
	const buttonClass =
		'inline-flex min-h-10 items-center justify-center gap-2 rounded-lg bg-accent px-4 text-sm font-semibold text-on-accent transition hover:opacity-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent disabled:opacity-60';
</script>

<section class="rise-in mt-10 rounded-[var(--radius-card)] border border-line bg-surface px-4 py-4 shadow-lift sm:px-5" style="--rise-delay: 200ms" aria-labelledby="updates">
	<h2 id="updates" class="text-base font-semibold tracking-tight text-ink">{m.status_updates_title()}</h2>
	{#if subscribe}
		<p class="mt-0.5 text-[0.8125rem] text-ink-2">{m.status_sub_lead()}</p>
		{#if step === 'done'}
			<p class="mt-3 flex items-start gap-2 text-sm text-ink" role="status">
				<Check class="mt-0.5 size-4 shrink-0 text-accent" aria-hidden="true" />
				{done}
			</p>
			{#if secret}
				<div class="mt-3 grid gap-1.5">
					<label for="subscribe-secret" class="text-sm font-semibold text-ink">{m.status_sub_secret_title()}</label>
					<input id="subscribe-secret" class="input tnum" readonly value={secret} onfocus={(event) => event.currentTarget.select()} />
					<p class="text-[0.8125rem] text-ink-2">{m.status_sub_secret_help()}</p>
				</div>
			{/if}
		{:else if step === 'code'}
			<p class="mt-3 flex items-start gap-2 text-sm text-ink" role="status">
				<Mail class="mt-0.5 size-4 shrink-0 text-accent" aria-hidden="true" />
				{done}
			</p>
			<form class="mt-3 flex flex-col gap-2 sm:flex-row" onsubmit={confirmCode} novalidate>
				<label for="subscribe-code" class="sr-only">{m.status_sub_code_label()}</label>
				<input
					id="subscribe-code"
					class={`${inputClass} tnum`}
					bind:value={code}
					inputmode="numeric"
					autocomplete="one-time-code"
					maxlength="6"
					placeholder="123456"
					disabled={sending}
					aria-invalid={error ? 'true' : undefined}
					aria-describedby={error ? 'subscribe-error' : undefined}
				/>
				<button type="submit" class={buttonClass} disabled={sending || code.trim().length < 6}>
					<Check class="size-4" aria-hidden="true" />
					{sending ? m.status_updates_sending() : m.status_sub_code_button()}
				</button>
			</form>
			{#if error}
				<p id="subscribe-error" class="mt-2 text-sm text-warning-ink" role="alert">{error}</p>
			{/if}
		{:else}
			{#if kinds.length > 1}
				<div class="mt-3 flex flex-wrap gap-1.5" role="group" aria-label={m.status_sub_tabs_label()}>
					{#each kinds as option (option)}
						<button
							type="button"
							class={`inline-flex min-h-10 items-center rounded-full border px-3.5 text-sm font-semibold ${kind === option ? 'border-ink bg-ink text-surface' : 'border-line text-ink-2 hover:text-ink'}`}
							aria-pressed={kind === option}
							onclick={() => {
								chosen = option;
								error = null;
							}}
						>
							{LABEL[option]()}
						</button>
					{/each}
				</div>
			{/if}
			<form class="mt-3 grid gap-3" onsubmit={submit} novalidate>
				<div class="flex flex-col gap-2 sm:flex-row">
					{#if kind === 'email'}
						<label for="subscribe-email" class="sr-only">{m.status_updates_email_label()}</label>
						<input id="subscribe-email" type="email" class={inputClass} bind:value={email} placeholder="you@example.org" autocomplete="email" maxlength="254" disabled={sending} aria-invalid={error ? 'true' : undefined} aria-describedby={error ? 'subscribe-error' : undefined} />
					{:else if kind === 'sms'}
						<label for="subscribe-phone" class="sr-only">{m.status_sub_phone_label()}</label>
						<input id="subscribe-phone" type="tel" class={inputClass} bind:value={phone} placeholder="+33612345678" autocomplete="tel" maxlength="24" disabled={sending} aria-invalid={error ? 'true' : undefined} aria-describedby={error ? 'subscribe-error' : undefined} />
					{:else}
						<label for="subscribe-url" class="sr-only">{kind === 'slack' ? m.status_sub_slack_label() : m.status_sub_webhook_label()}</label>
						<input id="subscribe-url" type="url" class={inputClass} bind:value={url} placeholder={kind === 'slack' ? 'https://hooks.slack.com/services/…' : 'https://example.org/hooks/status'} maxlength="500" disabled={sending} aria-invalid={error ? 'true' : undefined} aria-describedby={error ? 'subscribe-error' : undefined} />
					{/if}
					<button type="submit" class={buttonClass} disabled={sending}>
						<Mail class="size-4" aria-hidden="true" />
						{sending ? m.status_updates_sending() : m.status_updates_subscribe()}
					</button>
				</div>
				<p class="-mt-1 text-[0.8125rem] text-ink-2">
					{#if kind === 'sms'}{m.status_sub_phone_help()}{:else if kind === 'webhook'}{m.status_sub_webhook_help()}{:else if kind === 'slack'}{m.status_sub_slack_help()}{:else}{m.status_updates_email_lead()}{/if}
				</p>
				{#if services.length > 1}
					<fieldset class="grid gap-1" disabled={sending}>
						<legend class="text-sm font-semibold text-ink">{m.status_sub_services_legend()}</legend>
						<p class="text-[0.8125rem] text-ink-2">{m.status_sub_services_help()}</p>
						<div class="flex flex-wrap gap-x-4">
							{#each services as service (service.key)}
								<label class="inline-flex min-h-10 items-center gap-2 text-sm text-ink">
									<input
										type="checkbox"
										class="size-4 accent-[var(--c-signal)]"
										checked={picked.includes(service.key)}
										onchange={(event) => (picked = event.currentTarget.checked ? [...picked, service.key] : picked.filter((key) => key !== service.key))}
									/>
									{service.label}
								</label>
							{/each}
						</div>
					</fieldset>
				{/if}
			</form>
			{#if error}
				<p id="subscribe-error" class="mt-2 text-sm text-warning-ink" role="alert">{error}</p>
			{/if}
		{/if}
	{/if}
	<p class={`${subscribe ? 'mt-3' : 'mt-2'} text-sm text-ink-2`}>
		<a class="inline-flex items-center gap-1.5 font-semibold text-accent underline decoration-accent/40 underline-offset-4 hover:decoration-accent" href={rss}>
			<Rss class="size-4" aria-hidden="true" />
			{m.status_updates_rss()}
		</a>
	</p>
	<p class="mt-2 text-sm text-ink-2">
		<a class="inline-flex items-center gap-1.5 font-semibold text-accent underline decoration-accent/40 underline-offset-4 hover:decoration-accent" href={ical}>
			<CalendarDays class="size-4" aria-hidden="true" />
			{m.status_updates_ical()}
		</a>
	</p>
</section>
