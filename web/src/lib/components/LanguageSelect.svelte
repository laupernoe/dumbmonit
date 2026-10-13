<script lang="ts">
	/**
	 * Interface language picker: every locale under its own native name, plus
	 * "Automatic" (browser language). The choice lives in the browser (Paraglide
	 * localStorage strategy), so it also works on /login; it reloads the page,
	 * and the session cookie is untouched.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale, locales, setLocale, type Locale } from '#lib/paraglide/runtime.js';

	let { id, class: cls = '' }: { id: string; class?: string } = $props();

	const KEY = 'PARAGLIDE_LOCALE';
	const AUTO = 'auto';

	function stored(): boolean {
		try {
			return localStorage.getItem(KEY) !== null;
		} catch {
			return false;
		}
	}
	const current = stored() ? getLocale() : AUTO;

	function nativeName(locale: string): string {
		try {
			const name = new Intl.DisplayNames([locale], { type: 'language' }).of(locale) ?? locale;
			return name.charAt(0).toLocaleUpperCase(locale) + name.slice(1);
		} catch {
			return locale;
		}
	}

	function change(value: string) {
		if (value === AUTO) {
			try {
				localStorage.removeItem(KEY);
			} catch {
				/* storage blocked: nothing to forget */
			}
			location.reload();
		} else {
			setLocale(value as Locale);
		}
	}
</script>

<select
	{id}
	class={`h-10 min-w-0 max-w-full rounded-lg border border-line-strong bg-surface px-3 text-sm text-ink ${cls}`}
	value={current}
	onchange={(e) => change((e.currentTarget as HTMLSelectElement).value)}
>
	<option value={AUTO}>{m.settings_language_auto()}</option>
	{#each locales as locale (locale)}
		<option value={locale} lang={locale}>{nativeName(locale)}</option>
	{/each}
</select>
