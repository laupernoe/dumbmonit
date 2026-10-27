<script lang="ts">
	/**
	 * Settings → General: the product mode (Homelab or Enterprise, with a
	 * miniature of each), the instance's name, language and time zone, and a
	 * way back into the setup wizard. Switching modes changes the interface
	 * only: pages, monitors and settings stay as they are.
	 */
	import { goto } from '$app/navigation';
	import { getInstanceSettings, updateInstanceSettings } from '#lib/api/instance.js';
	import type { InstanceSettings, ProductMode } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { mode } from '#lib/stores/mode.svelte.js';
	import { Button, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import ModePicker from '#lib/components/modes/ModePicker.svelte';
	import InstanceFields from '#lib/components/modes/InstanceFields.svelte';

	let loaded = $state<InstanceSettings | null>(null);
	let loadError = $state<unknown>(null);
	let productMode = $state<ProductMode>(mode.current);
	let name = $state('');
	let language = $state('');
	let timezone = $state('');
	let savingMode = $state(false);
	let modeError = $state<unknown>(null);
	let saving = $state(false);
	let saveError = $state<unknown>(null);
	let saved = $state(false);

	const readOnly = $derived(!auth.isAdmin);
	const dirty = $derived(!!loaded && (name.trim() !== loaded.name || language !== loaded.language || timezone !== loaded.timezone));

	$effect(() => {
		const controller = new AbortController();
		getInstanceSettings(controller.signal)
			.then((settings) => {
				loaded = settings;
				productMode = settings.product_mode;
				name = settings.name;
				language = settings.language;
				timezone = settings.timezone;
			})
			.catch((cause) => {
				if (!(cause instanceof DOMException && cause.name === 'AbortError')) loadError = cause;
			});
		return () => controller.abort();
	});

	/** The mode is saved on the spot: the shell changes under the reader's eyes. */
	async function changeMode(next: ProductMode) {
		const previous = mode.current;
		savingMode = true;
		modeError = null;
		try {
			const settings = await updateInstanceSettings({ product_mode: next });
			loaded = settings;
			mode.set(settings.product_mode);
		} catch (cause) {
			productMode = previous;
			modeError = cause;
		} finally {
			savingMode = false;
		}
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (saving || readOnly) return;
		saving = true;
		saveError = null;
		saved = false;
		try {
			const settings = await updateInstanceSettings({ name: name.trim(), language, timezone });
			loaded = settings;
			name = settings.name;
			mode.name = settings.name;
			saved = true;
		} catch (cause) {
			saveError = cause;
		} finally {
			saving = false;
		}
	}
</script>

<Panel id="product-mode" title={m.settings_general_mode_title()} description={m.settings_general_mode_text()}>
	{#if loadError}
		<ErrorNotice error={loadError} />
	{:else if !loaded}
		<Skeleton class="h-64" />
	{:else}
		<ModePicker bind:value={productMode} onchange={changeMode} disabled={readOnly || savingMode} name="settings-product-mode" />
		<p class="mt-3 text-[0.8125rem] text-ink-2" aria-live="polite">
			{savingMode ? m.settings_general_mode_saving() : m.mode_switch_keeps()}
		</p>
		{#if modeError}<ErrorNotice error={modeError} class="mt-3" />{/if}
		{#if readOnly}<p class="mt-2 text-[0.8125rem] text-ink-2">{m.settings_general_readonly()}</p>{/if}
	{/if}
</Panel>

<div class="mt-6 max-sm:mt-3">
	<Panel id="general" title={m.settings_general_instance_title()} description={m.settings_general_instance_text()}>
		{#if loaded}
			<form onsubmit={save} class="grid gap-4">
				<InstanceFields bind:name bind:language bind:timezone disabled={readOnly} idPrefix="settings-instance" />
				{#if saveError}<ErrorNotice error={saveError} />{/if}
				{#if !readOnly}
					<div class="flex flex-wrap items-center gap-3">
						<Button type="submit" variant="primary" loading={saving} disabled={!dirty}>{m.settings_general_save()}</Button>
						{#if saved && !dirty}<Plate tone="signal" draw>{m.settings_general_saved()}</Plate>{/if}
					</div>
				{/if}
			</form>
		{:else if !loadError}
			<Skeleton class="h-32" />
		{/if}
	</Panel>
</div>

{#if auth.isAdmin}
	<div class="mt-6 max-sm:mt-3">
		<Panel id="setup-wizard" title={m.settings_general_wizard_title()} description={m.settings_general_wizard_text()}>
			<Button onclick={() => goto('/welcome')}>{m.settings_general_wizard_run()}</Button>
		</Panel>
	</div>
{/if}
