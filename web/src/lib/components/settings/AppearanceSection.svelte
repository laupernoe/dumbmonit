<script lang="ts">
	/**
	 * Settings → My account → Appearance: the language. The console has one
	 * dark palette, so there is no theme to pick here.
	 */
	import { Panel } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import LanguageSelect from '#lib/components/LanguageSelect.svelte';
</script>

<Panel id="appearance" title={m.settings_appearance_language_title()} description={m.settings_language_theme_description()}>
	<div class="flex flex-wrap items-center justify-between gap-3 max-sm:flex-nowrap">
		<label for="settings-language" class="text-sm font-semibold text-ink">{m["settings.appearance.language_label"]()}</label>
		<LanguageSelect id="settings-language" class="w-full max-sm:w-auto max-sm:min-w-0 max-sm:flex-1 sm:w-auto sm:min-w-56" />
	</div>
</Panel>
