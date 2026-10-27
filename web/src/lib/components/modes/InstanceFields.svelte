<script lang="ts">
	/**
	 * Instance name, default language and time zone: the second step of the
	 * setup wizard and the "Instance" block of Settings → General.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { locales } from '#lib/paraglide/runtime.js';
	import { Field } from '#lib/ui/index.js';
	import { timeZones } from './timezones.js';

	interface Props {
		name: string;
		language: string;
		timezone: string;
		disabled?: boolean;
		/** Prefix of the field ids, unique per page. */
		idPrefix?: string;
	}
	let { name = $bindable(), language = $bindable(), timezone = $bindable(), disabled = false, idPrefix = 'instance' }: Props = $props();

	const zones = timeZones();

	function nativeName(locale: string): string {
		try {
			const label = new Intl.DisplayNames([locale], { type: 'language' }).of(locale) ?? locale;
			return label.charAt(0).toLocaleUpperCase(locale) + label.slice(1);
		} catch {
			return locale;
		}
	}
</script>

<div class="grid gap-4 sm:grid-cols-2">
	<Field label={m.wizard_instance_name()} for={`${idPrefix}-name`} help={m.wizard_instance_name_help()} class="sm:col-span-2">
		<input id={`${idPrefix}-name`} class="input" maxlength="80" autocomplete="off" placeholder={m.wizard_instance_name_placeholder()} bind:value={name} {disabled} />
	</Field>
	<Field label={m.wizard_instance_language()} for={`${idPrefix}-language`}>
		<select id={`${idPrefix}-language`} class="input" bind:value={language} {disabled}>
			<option value="">{m.wizard_instance_language_auto()}</option>
			{#each locales as locale (locale)}
				<option value={locale} lang={locale}>{nativeName(locale)}</option>
			{/each}
		</select>
	</Field>
	<Field label={m.wizard_instance_timezone()} for={`${idPrefix}-timezone`}>
		<select id={`${idPrefix}-timezone`} class="input" bind:value={timezone} {disabled}>
			<option value="">UTC</option>
			{#each zones as zone (zone)}
				{#if zone !== 'UTC'}<option value={zone}>{zone.replace(/_/g, ' ')}</option>{/if}
			{/each}
		</select>
	</Field>
</div>
