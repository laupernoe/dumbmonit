<script lang="ts">
	/**
	 * Setup wizard, after the first administrator signs in (and again from
	 * Settings → General): mode, name and language, a notification channel
	 * with a test, first monitors, an agent — and, in Enterprise mode, three
	 * optional steps more (single sign-on and teams, on-call, retention).
	 *
	 * Every step can be skipped; "Skip setup" ends it at once. The step lives
	 * in the URL (`?step=channel`), so Back works and a language change, which
	 * reloads the page, lands on the next step. Finishing marks the wizard done
	 * on the server, for every browser.
	 */
	import { tick, untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Check } from 'lucide-svelte';
	import { getInstanceSettings, updateInstanceSettings } from '#lib/api/instance.js';
	import { updateOnboarding, type ProductMode } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale, setLocale, type Locale } from '#lib/paraglide/runtime.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { mode } from '#lib/stores/mode.svelte.js';
	import { Button, ErrorNotice } from '#lib/ui/index.js';
	import Logo from '#lib/components/Logo.svelte';
	import ModePicker from '#lib/components/modes/ModePicker.svelte';
	import InstanceFields from '#lib/components/modes/InstanceFields.svelte';
	import { browserTimeZone } from '#lib/components/modes/timezones.js';
	import WizardChannel from '#lib/components/wizard/WizardChannel.svelte';
	import WizardMonitors from '#lib/components/wizard/WizardMonitors.svelte';
	import WizardOnCall from '#lib/components/wizard/WizardOnCall.svelte';
	import AgentEnroll from '#lib/components/device-form/AgentEnroll.svelte';
	import SsoSection from '#lib/components/settings/SsoSection.svelte';
	import UsersSection from '#lib/components/settings/UsersSection.svelte';
	import StorageSection from '#lib/components/settings/StorageSection.svelte';

	type StepId = 'mode' | 'instance' | 'channel' | 'monitors' | 'agent' | 'sso' | 'oncall' | 'retention';
	interface Step {
		id: StepId;
		title: () => string;
		text: () => string;
		optional?: boolean;
	}

	const BASE: Step[] = [
		{ id: 'mode', title: () => m.wizard_mode_title(), text: () => m.wizard_mode_text() },
		{ id: 'instance', title: () => m.wizard_instance_title(), text: () => m.wizard_instance_text() },
		{ id: 'channel', title: () => m.wizard_channel_title(), text: () => m.wizard_channel_text() },
		{ id: 'monitors', title: () => m.wizard_monitors_title(), text: () => m.wizard_monitors_text() },
		{ id: 'agent', title: () => m.wizard_agent_title(), text: () => m.wizard_agent_text() }
	];
	const ENTERPRISE: Step[] = [
		{ id: 'sso', title: () => m.wizard_sso_title(), text: () => m.wizard_sso_text(), optional: true },
		{ id: 'oncall', title: () => m.wizard_oncall_title(), text: () => m.wizard_oncall_text(), optional: true },
		{ id: 'retention', title: () => m.wizard_retention_title(), text: () => m.wizard_retention_text(), optional: true }
	];

	let chosen = $state<ProductMode>(untrack(() => mode.current));
	const steps = $derived(chosen === 'enterprise' ? [...BASE, ...ENTERPRISE] : BASE);
	const asked = $derived(page.url.searchParams.get('step'));
	const index = $derived(Math.max(0, steps.findIndex((s) => s.id === asked)));
	const step = $derived(steps[index]);
	const last = $derived(index === steps.length - 1);

	let name = $state('');
	let language = $state('');
	let timezone = $state('');
	let busy = $state(false);
	let error = $state<unknown>(null);
	let heading = $state<HTMLHeadingElement | null>(null);

	$effect(() => {
		getInstanceSettings()
			.then((settings) => {
				chosen = settings.product_mode;
				name = settings.name;
				language = settings.language;
				timezone = settings.timezone || browserTimeZone();
			})
			.catch(() => {
				timezone = browserTimeZone();
			});
	});

	// Only administrators set the instance up.
	$effect(() => {
		if (auth.canUseApi && !auth.isAdmin) void goto('/', { replace: true });
	});

	// Each new step takes the focus to its title, so a screen reader hears where it is.
	$effect(() => {
		step;
		void tick().then(() => heading?.focus());
	});

	function go(target: number) {
		const next = steps[Math.max(0, Math.min(steps.length - 1, target))];
		error = null;
		void goto(`/welcome?step=${next.id}`, { reset: false }).then(() => window.scrollTo({ top: 0 }));
	}

	/** Saves what the step holds, then moves on. */
	async function advance() {
		if (busy) return;
		busy = true;
		error = null;
		try {
			if (step.id === 'mode') {
				if (chosen !== mode.current) {
					const settings = await updateInstanceSettings({ product_mode: chosen });
					mode.set(settings.product_mode);
				}
			} else if (step.id === 'instance') {
				const settings = await updateInstanceSettings({ name: name.trim(), language, timezone });
				mode.name = settings.name;
				if (settings.language && settings.language !== getLocale()) {
					// A new language reloads the page: land on the next step.
					await goto(`/welcome?step=${steps[index + 1].id}`, { replace: true });
					setLocale(settings.language as Locale);
					return;
				}
			}
			if (last) await finish();
			else go(index + 1);
		} catch (cause) {
			error = cause;
		} finally {
			busy = false;
		}
	}

	/** Marks the wizard done (and the overview's three-step guide with it), then opens the app. */
	async function finish() {
		busy = true;
		error = null;
		try {
			await updateInstanceSettings({ setup_completed: true });
			mode.setupCompleted = true;
			await updateOnboarding({ skipped: true }).catch(() => {});
			await goto('/', { replace: true });
		} catch (cause) {
			error = cause;
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>{m.wizard_title()} · DumbMonit</title></svelte:head>

<div class="min-h-dvh bg-canvas text-ink">
	<header class="border-b border-line">
		<div class="mx-auto flex h-14 max-w-5xl items-center gap-3 px-4 sm:px-6">
			<Logo class="size-7" />
			<span class="truncate font-semibold">{m.wizard_title()}</span>
			<Button variant="ghost" size="sm" class="ml-auto !h-10" disabled={busy} onclick={finish}>{m.wizard_skip_all()}</Button>
		</div>
	</header>

	<div class="mx-auto grid max-w-5xl gap-8 px-4 py-6 sm:px-6 sm:py-10 lg:grid-cols-[14rem_minmax(0,1fr)]">
		<nav aria-label={m.wizard_steps_label()} class="max-lg:hidden">
			<ol class="grid gap-1">
				{#each steps as s, i (s.id)}
					{@const done = i < index}
					{@const current = i === index}
					<li>
						<button
							type="button"
							onclick={() => go(i)}
							aria-current={current ? 'step' : undefined}
							class={`flex min-h-10 w-full items-center gap-3 rounded-lg px-2 text-left text-sm transition-colors hover:bg-surface-2 ${current ? 'font-semibold text-ink' : 'text-ink-2'}`}
						>
							<span aria-hidden="true" class={`grid size-6 shrink-0 place-items-center rounded-full border font-mono text-xs ${current ? 'border-ink bg-ink text-canvas' : done ? 'border-line-strong text-ink' : 'border-line text-ink-3'}`}>
								{#if done}<Check class="size-3.5" />{:else}{i + 1}{/if}
							</span>
							<span class="min-w-0 flex-1 truncate">{s.title()}</span>
							{#if s.optional}<span class="text-[0.6875rem] text-ink-3">{m.wizard_optional()}</span>{/if}
						</button>
					</li>
				{/each}
			</ol>
			<p class="mt-6 px-2 text-[0.8125rem] text-ink-2">{m.wizard_intro()}</p>
		</nav>

		<section class="min-w-0" aria-labelledby="wizard-step-title">
			<div class="lg:hidden">
				<p class="text-[0.8125rem] text-ink-2">{m.wizard_step_of({ current: index + 1, total: steps.length })}</p>
				<div class="mt-2 h-1 overflow-hidden rounded-full bg-line" aria-hidden="true">
					<div class="h-full rounded-full bg-ink transition-[width] duration-500 ease-out-expo" style={`width: ${((index + 1) / steps.length) * 100}%`}></div>
				</div>
			</div>

			{#key step.id}
				<div class="rise-in mt-4 lg:mt-0">
					<h1 id="wizard-step-title" bind:this={heading} tabindex="-1" class="display text-3xl text-ink outline-none max-sm:text-2xl">
						{step.title()}
						{#if step.optional}<span class="ml-2 align-middle text-sm font-normal text-ink-3">{m.wizard_optional()}</span>{/if}
					</h1>
					<p class="mt-2 max-w-[65ch] text-ink-2">{step.text()}</p>

					<div class="mt-6 min-w-0">
						{#if step.id === 'mode'}
							<ModePicker bind:value={chosen} name="wizard-product-mode" />
							<p class="mt-3 text-[0.8125rem] text-ink-2">{m.mode_switch_keeps()}</p>
						{:else if step.id === 'instance'}
							<InstanceFields bind:name bind:language bind:timezone idPrefix="wizard-instance" />
						{:else if step.id === 'channel'}
							<WizardChannel />
						{:else if step.id === 'monitors'}
							<WizardMonitors />
						{:else if step.id === 'agent'}
							<AgentEnroll cancelHref={`/welcome?step=${steps[index + 1]?.id ?? 'agent'}`} />
						{:else if step.id === 'sso'}
							<div class="grid gap-6">
								<SsoSection />
								<UsersSection />
							</div>
						{:else if step.id === 'oncall'}
							<WizardOnCall {timezone} />
						{:else if step.id === 'retention'}
							<StorageSection />
						{/if}
					</div>
				</div>
			{/key}

			{#if error}<ErrorNotice {error} class="mt-6" />{/if}

			<div class="sticky bottom-0 mt-8 flex flex-wrap items-center gap-2 border-t border-line bg-canvas py-4 pb-[calc(1rem+var(--safe-bottom))]">
				{#if index > 0}
					<Button variant="ghost" class="!h-10" disabled={busy} onclick={() => go(index - 1)}>{m.wizard_back()}</Button>
				{/if}
				<div class="ml-auto flex flex-wrap items-center gap-2">
					<Button variant="ghost" class="!h-10" disabled={busy} onclick={() => (last ? finish() : go(index + 1))}>{m.wizard_skip_step()}</Button>
					<Button variant="primary" class="!h-10" loading={busy} onclick={advance}>{last ? m.wizard_finish() : m.wizard_next()}</Button>
				</div>
			</div>
		</section>
	</div>
</div>
