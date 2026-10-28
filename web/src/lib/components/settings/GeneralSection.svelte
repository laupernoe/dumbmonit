<script lang="ts">
	/**
	 * Settings → General: the instance's name, language and time zone, and a
	 * way back into the setup wizard.
	 */
	import { goto } from '$app/navigation';
	import { getInstanceSettings, updateInstanceSettings } from '#lib/api/instance.js';
	import type { InstanceSettings } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { instanceStore } from '#lib/stores/instance.svelte.js';
	import { Button, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import InstanceFields from '#lib/components/modes/InstanceFields.svelte';

	let loaded = $state<InstanceSettings | null>(null);
	let loadError = $state<unknown>(null);
	let name = $state('');
	let language = $state('');
	let timezone = $state('');
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
				name = settings.name;
				language = settings.language;
				timezone = settings.timezone;
			})
			.catch((cause) => {
				if (!(cause instanceof DOMException && cause.name === 'AbortError')) loadError = cause;
			});
		return () => controller.abort();
	});

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
			instanceStore.name = settings.name;
			saved = true;
		} catch (cause) {
			saveError = cause;
		} finally {
			saving = false;
		}
	}
</script>

<Panel id="general" title={m.settings_general_instance_title()} description={m.settings_general_instance_text()}>
	{#if loadError}
		<ErrorNotice error={loadError} />
	{:else if loaded}
		<form onsubmit={save} class="grid gap-4">
			<InstanceFields bind:name bind:language bind:timezone disabled={readOnly} idPrefix="settings-instance" />
			{#if saveError}<ErrorNotice error={saveError} />{/if}
			{#if readOnly}
				<p class="text-[0.8125rem] text-ink-2">{m.settings_general_readonly()}</p>
			{:else}
				<div class="flex flex-wrap items-center gap-3">
					<Button type="submit" variant="primary" loading={saving} disabled={!dirty}>{m.settings_general_save()}</Button>
					{#if saved && !dirty}<Plate tone="signal" draw>{m.settings_general_saved()}</Plate>{/if}
				</div>
			{/if}
		</form>
	{:else}
		<Skeleton class="h-32" />
	{/if}
</Panel>

{#if auth.isAdmin}
	<div class="mt-6 max-sm:mt-3">
		<Panel id="setup-wizard" title={m.settings_general_wizard_title()} description={m.settings_general_wizard_text()}>
			<Button onclick={() => goto('/welcome')}>{m.settings_general_wizard_run()}</Button>
		</Panel>
	</div>
{/if}
