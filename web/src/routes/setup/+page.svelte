<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * First run: create the admin account that protects the instance.
	 *
	 * Shown only while the server answers `configured: false`. Creating the
	 * account also opens the session, and the layout guard then moves to the
	 * overview by itself.
	 */
	import { ApiError } from '#lib/api/index.js';
	import { auth, PASSWORD_MIN_LENGTH, validatePassword } from '#lib/stores/auth.svelte.js';
	import { Button, ClickSpark, DotField, ErrorNotice, Field } from '#lib/ui/index.js';
	import Logo from '#lib/components/Logo.svelte';
	import ThemeToggle from '#lib/components/ThemeToggle.svelte';
	import PasswordInput from '#lib/components/settings/PasswordInput.svelte';
	import Mascot from '#lib/components/Mascot.svelte';

	let setupCode = $state('');
	let codeError = $state<string | null>(null);
	let username = $state('admin');
	let password = $state('');
	let confirmation = $state('');
	let sending = $state(false);
	let usernameError = $state<string | null>(null);
	let passwordError = $state<string | null>(null);
	let confirmError = $state<string | null>(null);
	let failure = $state<{ title: string; error: unknown } | null>(null);

	// Code points, as the server counts them.
	const remaining = $derived(Math.max(0, PASSWORD_MIN_LENGTH - [...password].length));
	const lengthHelp = $derived(
		password.length === 0
			? m.auth_setup_length_empty({ min: PASSWORD_MIN_LENGTH })
			: remaining > 0
				? m.auth_setup_length_remaining({ count: remaining, min: PASSWORD_MIN_LENGTH })
				: m.auth_setup_length_ok()
	);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		failure = null;
		const name = username.trim();
		const code = setupCode.trim();
		codeError = code ? null : m.auth_setup_code_required();
		usernameError = !name ? m.auth_setup_username_required() : /\s/.test(name) ? m.auth_setup_username_spaces() : null;
		passwordError = validatePassword(password);
		confirmError = !passwordError && password !== confirmation ? m.auth_setup_mismatch() : null;
		if (codeError || usernameError || passwordError || confirmError) return;

		sending = true;
		try {
			await auth.setupAccount(code, name, password);
			password = '';
			confirmation = '';
		} catch (cause) {
			if (cause instanceof ApiError && cause.status === 409) {
				failure = {
					title: m.auth_setup_exists_title(),
					error: new ApiError(m.auth_setup_exists_detail(), 400)
				};
				// The guard will move to the sign-in screen once the status is re-read.
				await auth.refresh();
			} else if (cause instanceof ApiError && cause.status === 401) {
				codeError = m.auth_setup_code_wrong();
			} else {
				failure = { title: m.auth_setup_failed(), error: cause };
			}
		} finally {
			sending = false;
		}
	}
</script>

<svelte:head><title>{m.auth_setup_page_title()}</title></svelte:head>

<div class="relative min-h-screen overflow-hidden bg-canvas">
	<DotField opacity={1} dotSpacing={13} />

	<div class="absolute top-3 right-3 z-20 sm:top-4 sm:right-4">
		<ThemeToggle />
	</div>

	<main class="relative z-10 flex min-h-screen flex-col items-center justify-center px-4 py-12">
		<section class="gate-panel rise-in relative w-full max-w-sm rounded-[var(--radius-card)] border border-line p-6 shadow-float sm:p-7" aria-labelledby="gate-title">
			<!-- The pigeon peeks over the corner: the only playful note on an otherwise plain gate. -->
			<Mascot mood="happy" class="mascot pointer-events-none absolute -top-12 -right-4 size-20 rotate-6 sm:-top-16 sm:-right-6 sm:size-28" />
			<div class="flex items-center gap-2.5">
				<Logo class="size-8" />
				<span class="text-[1.05rem] font-bold tracking-tight text-ink">DumbMonit</span>
			</div>

			<h1 id="gate-title" class="display mt-6 text-[1.75rem] text-ink sm:text-3xl">{m.auth_setup_title()}</h1>
			<p class="mt-2 text-sm text-ink-2">
				{m.auth_setup_intro({ min: PASSWORD_MIN_LENGTH })}
			</p>

			<form class="mt-6 grid gap-4" onsubmit={submit} novalidate>
				<Field
					label={m.auth_setup_code_label()}
					for="setup-code"
					error={codeError}
					help={m.auth_setup_code_help()}
				>
					<input
						id="setup-code"
						type="text"
						class="input font-mono tracking-wider uppercase"
						bind:value={setupCode}
						autocomplete="one-time-code"
						autocapitalize="characters"
						spellcheck="false"
						placeholder="XXXXX-XXXXX"
						disabled={sending}
						aria-invalid={codeError ? 'true' : undefined}
						oninput={() => (codeError = null)}
					/>
				</Field>

				<Field label={m.auth_login_username()} for="username" error={usernameError}>
					<input
						id="username"
						type="text"
						class="input"
						bind:value={username}
						autocomplete="username"
						autocapitalize="off"
						spellcheck="false"
						disabled={sending}
						aria-invalid={usernameError ? 'true' : undefined}
						oninput={() => (usernameError = null)}
					/>
				</Field>

				<Field label={m.auth_login_password()} for="new-password" error={passwordError} help={lengthHelp}>
					<PasswordInput
						id="new-password"
						bind:value={password}
						autocomplete="new-password"
						disabled={sending}
						invalid={passwordError !== null}
						oninput={() => (passwordError = null)}
					/>
				</Field>

				<Field label={m.auth_setup_confirm()} for="confirm-password" error={confirmError}>
					<PasswordInput
						id="confirm-password"
						bind:value={confirmation}
						autocomplete="new-password"
						disabled={sending}
						invalid={confirmError !== null}
						oninput={() => (confirmError = null)}
					/>
				</Field>

				{#if failure}
					<ErrorNotice title={failure.title} error={failure.error} />
				{/if}

				<ClickSpark class="w-full">
					<Button type="submit" variant="primary" size="lg" class="w-full" loading={sending}>
						{m.auth_setup_submit()}
					</Button>
				</ClickSpark>
			</form>

			<p class="mt-5 text-[0.8125rem] text-ink-2">
				{m.auth_setup_note()}
			</p>
		</section>

		<p class="mt-6 text-[0.8125rem] text-ink-2">{m.auth_footer()}</p>
	</main>
</div>

<style>
	/* The mascot casts the same soft shadow as the panel, as if it sat on it. */
	:global(.mascot) {
		filter: drop-shadow(0 10px 18px rgb(0 0 0 / 0.18));
	}

	.gate-panel {
		background:
			linear-gradient(180deg, rgb(255 255 255 / 0.04), rgb(0 0 0 / 0.02)),
			repeating-linear-gradient(180deg, transparent 0 3px, rgb(127 127 127 / 0.025) 3px 4px),
			color-mix(in srgb, var(--c-surface) 95%, transparent);
		backdrop-filter: blur(10px);
		-webkit-backdrop-filter: blur(10px);
	}
</style>
