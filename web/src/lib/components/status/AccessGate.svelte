<script lang="ts">
	/**
	 * What a visitor sees instead of a private status page: a password form, or
	 * an invitation to sign in to DumbMonit. Once the password is accepted the
	 * server has set its access cookie, and `onunlocked` reloads the page.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { Lock, LogIn } from 'lucide-svelte';
	import { toApiError, unlockStatusPage } from '#lib/api/index.js';
	import { Button, Field } from '#lib/ui/index.js';

	interface Props {
		slug: string;
		access: 'password' | 'users';
		onunlocked: () => void;
	}

	let { slug, access, onunlocked }: Props = $props();

	let password = $state('');
	let busy = $state(false);
	let error = $state<string | null>(null);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!password || busy) return;
		busy = true;
		error = null;
		try {
			await unlockStatusPage(slug, password);
			password = '';
			onunlocked();
		} catch (cause) {
			const failure = toApiError(cause);
			// A wrong password is ours to word; a throttle comes with its own wait time.
			error = failure.status === 403 ? m.status_gate_password_wrong() : failure.message;
		} finally {
			busy = false;
		}
	}

	const login = $derived(`/login?redirect=${encodeURIComponent(`/s/${slug}`)}`);
</script>

<section class="rise-in mx-auto grid max-w-sm gap-4 rounded-[var(--radius-card)] border border-line bg-surface p-6 shadow-lift">
	{#if access === 'password'}
		<div class="flex items-center gap-2.5">
			<Lock class="size-5 text-ink-2" aria-hidden="true" />
			<h1 class="text-lg font-semibold text-ink">{m.status_gate_password_title()}</h1>
		</div>
		<p class="text-sm text-ink-2">{m.status_gate_password_help()}</p>
		<form class="grid gap-3" onsubmit={submit}>
			<Field label={m.status_gate_password_label()} for="gate-password" {error}>
				<input
					id="gate-password"
					type="password"
					class="input"
					bind:value={password}
					autocomplete="current-password"
					disabled={busy}
					aria-invalid={error ? 'true' : undefined}
				/>
			</Field>
			<Button type="submit" loading={busy} disabled={!password}>{m.status_gate_password_submit()}</Button>
		</form>
	{:else}
		<div class="flex items-center gap-2.5">
			<LogIn class="size-5 text-ink-2" aria-hidden="true" />
			<h1 class="text-lg font-semibold text-ink">{m.status_gate_users_title()}</h1>
		</div>
		<p class="text-sm text-ink-2">{m.status_gate_users_help()}</p>
		<Button href={login}>{m.status_gate_users_action()}</Button>
	{/if}
</section>
