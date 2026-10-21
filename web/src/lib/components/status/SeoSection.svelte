<script lang="ts">
	/**
	 * "SEO" block of a published status page: whether search engines may index
	 * it, the texts they and link previews show (generated from the page name
	 * when left empty, shown as placeholders), a live "search result" preview,
	 * and a round trip through an AI assistant: copy a ready prompt, paste back
	 * its JSON answer to fill the fields.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { Check, ClipboardCopy, ClipboardPaste, Sparkles } from 'lucide-svelte';
	import type { StatusPageSeo } from '#lib/api/index.js';
	import { Button, Field, Panel, Toggle } from '#lib/ui/index.js';
	import { copyText } from '#lib/clipboard.js';
	import { getLocale, locales } from '#lib/paraglide/runtime.js';
	import {
		DESCRIPTION_TARGET,
		TITLE_TARGET,
		breadcrumb,
		buildPrompt,
		effectiveSeo,
		parseSeoReply,
		promptLocale,
		seoDefaults,
		truncate,
		type PromptTexts,
		type SeoProblem,
		type SeoTextKey
	} from './seo';

	interface Props {
		idPrefix: string;
		seo: StatusPageSeo;
		/** Page name, free description, address and logo address (empty when unknown). */
		name: string;
		description: string;
		url: string;
		logo: string;
		/** Public names of the monitors listed on the page. */
		services: string[];
		disabled?: boolean;
	}

	let { idPrefix, seo = $bindable(), name, description, url, logo, services, disabled = false }: Props = $props();

	const defaults = $derived(seoDefaults({ name, description, url, logo }));
	const shown = $derived(effectiveSeo(seo, defaults));
	const count = (text: string) => [...text.trim()].length;

	let copied = $state(false);
	let copyFailed = $state(false);
	let pasteOpen = $state(false);
	let pasted = $state('');
	let pasteError = $state<string | null>(null);
	let pasteNotes = $state<string[]>([]);
	let applied = $state(false);

	/** Prompt wording in the page language when the interface has it, else in the interface language. */
	function promptTexts(): PromptTexts {
		const locale = promptLocale(shown.lang, locales, getLocale());
		const o = { locale: locale as (typeof locales)[number] };
		return {
			intro: m.status_seo_prompt_intro({}, o),
			context: m.status_seo_prompt_context({}, o),
			name: (value) => m.status_seo_prompt_name({ value }, o),
			unnamed: m.status_seo_prompt_unnamed({}, o),
			organisation: (value) => m.status_seo_prompt_organisation({ value }, o),
			orgUnknown: m.status_seo_prompt_org_unknown({}, o),
			address: (value) => m.status_seo_prompt_address({ value }, o),
			addressUnknown: m.status_seo_prompt_address_unknown({}, o),
			language: (value) => m.status_seo_prompt_language({ value }, o),
			description: (value) => m.status_seo_prompt_description({ value }, o),
			services: (value) => m.status_seo_prompt_services({ value }, o),
			none: m.status_seo_prompt_none({}, o),
			more: (count) => m.status_seo_prompt_more({ count }, o),
			rules: m.status_seo_prompt_rules({}, o),
			ruleLang: (lang) => m.status_seo_prompt_rule_lang({ lang }, o),
			ruleTitle: (max) => m.status_seo_prompt_rule_title({ max }, o),
			ruleDescription: (max) => m.status_seo_prompt_rule_description({ max }, o),
			ruleKeywords: m.status_seo_prompt_rule_keywords({}, o),
			ruleCanonical: m.status_seo_prompt_rule_canonical({}, o),
			ruleImage: m.status_seo_prompt_rule_image({}, o),
			ruleLangField: m.status_seo_prompt_rule_lang_field({}, o),
			ruleSiteName: m.status_seo_prompt_rule_site_name({}, o),
			rulePlain: m.status_seo_prompt_rule_plain({}, o),
			reply: m.status_seo_prompt_reply({}, o)
		};
	}

	async function copyPrompt() {
		copyFailed = false;
		const ok = await copyText(
			buildPrompt({ name, organisation: shown.site_name, services, lang: shown.lang, url: shown.canonical, description }, promptTexts())
		);
		copied = ok;
		copyFailed = !ok;
		if (ok) setTimeout(() => (copied = false), 2500);
	}

	const FIELD_LABEL: Record<SeoTextKey, () => string> = {
		title: () => m.status_seo_title_label(),
		description: () => m.status_seo_description_label(),
		keywords: () => m.status_seo_keywords_label(),
		canonical: () => m.status_seo_canonical_label(),
		image: () => m.status_seo_image_label(),
		lang: () => m.status_seo_lang_label(),
		site_name: () => m.status_seo_site_name_label()
	};

	function problemText(problem: SeoProblem): string {
		const field = FIELD_LABEL[problem.field]();
		switch (problem.reason) {
			case 'invalid_url':
				return m.status_seo_paste_bad_url({ field });
			case 'invalid_lang':
				return m.status_seo_paste_bad_lang({ field });
			case 'too_long':
				return m.status_seo_paste_too_long({ field });
			default:
				return m.status_seo_paste_bad_type({ field });
		}
	}

	function applyPasted() {
		applied = false;
		pasteNotes = [];
		const result = parseSeoReply(pasted);
		if (!result.ok) {
			pasteError = {
				empty: m.status_seo_paste_empty(),
				not_json: m.status_seo_paste_not_json(),
				not_object: m.status_seo_paste_not_object(),
				no_fields: m.status_seo_paste_no_fields()
			}[result.reason];
			return;
		}
		pasteError = null;
		seo = { ...seo, ...result.values };
		pasteNotes = result.problems.map(problemText);
		applied = true;
		pasted = '';
	}

	const COUNT_OK = 'text-[0.75rem] tnum text-ink-3';
	const COUNT_OVER = 'text-[0.75rem] tnum text-warning';
</script>

<Panel title={m.status_seo_section()} description={m.status_seo_section_help()}>
	<div class="grid gap-5">
		<div class="flex items-start justify-between gap-4 rounded-lg border border-line px-4 py-3">
			<div class="min-w-0">
				<label for="{idPrefix}-seo-index" class="block text-sm font-semibold text-ink">{m.status_seo_index()}</label>
				<p class="text-[0.8125rem] text-ink-2">{seo.index ? m.status_seo_index_on() : m.status_seo_index_off()}</p>
			</div>
			<Toggle id="{idPrefix}-seo-index" bind:checked={seo.index} {disabled} label={m.status_seo_index()} />
		</div>

		<div class="grid items-start gap-x-8 gap-y-5 lg:grid-cols-2">
			<div class="grid content-start gap-4">
				<Field label={m.status_seo_title_label()} for="{idPrefix}-seo-title">
					<input id="{idPrefix}-seo-title" type="text" class="input" bind:value={seo.title} placeholder={defaults.title} maxlength="120" {disabled} />
					<span class={count(shown.title) > TITLE_TARGET ? COUNT_OVER : COUNT_OK}>{m.status_seo_counter({ count: count(shown.title), max: TITLE_TARGET })}</span>
				</Field>
				<Field label={m.status_seo_description_label()} for="{idPrefix}-seo-description">
					<textarea id="{idPrefix}-seo-description" class="input min-h-20" rows="3" bind:value={seo.description} placeholder={defaults.description} maxlength="320" {disabled}></textarea>
					<span class={count(shown.description) > DESCRIPTION_TARGET ? COUNT_OVER : COUNT_OK}>{m.status_seo_counter({ count: count(shown.description), max: DESCRIPTION_TARGET })}</span>
				</Field>
				<Field label={m.status_seo_keywords_label()} for="{idPrefix}-seo-keywords" help={m.status_seo_keywords_help()}>
					<input id="{idPrefix}-seo-keywords" type="text" class="input" bind:value={seo.keywords} placeholder={defaults.keywords} maxlength="300" {disabled} />
				</Field>
				<Field label={m.status_seo_site_name_label()} for="{idPrefix}-seo-site-name">
					<input id="{idPrefix}-seo-site-name" type="text" class="input" bind:value={seo.site_name} placeholder={defaults.site_name} maxlength="100" {disabled} />
				</Field>
				<div class="grid gap-4 sm:grid-cols-2">
					<Field label={m.status_seo_lang_label()} for="{idPrefix}-seo-lang" help={m.status_seo_lang_help()}>
						<input id="{idPrefix}-seo-lang" type="text" class="input font-mono" bind:value={seo.lang} placeholder={defaults.lang} maxlength="35" autocomplete="off" spellcheck="false" {disabled} />
					</Field>
				</div>
				<Field label={m.status_seo_canonical_label()} for="{idPrefix}-seo-canonical" help={m.status_seo_canonical_help()}>
					<input id="{idPrefix}-seo-canonical" type="url" class="input font-mono" bind:value={seo.canonical} placeholder={defaults.canonical || 'https://status.example.com/'} autocomplete="off" spellcheck="false" {disabled} />
				</Field>
				<Field label={m.status_seo_image_label()} for="{idPrefix}-seo-image" help={m.status_seo_image_help()}>
					<input id="{idPrefix}-seo-image" type="url" class="input font-mono" bind:value={seo.image} placeholder={defaults.image || 'https://example.com/share.png'} autocomplete="off" spellcheck="false" {disabled} />
				</Field>
			</div>

			<div class="grid content-start gap-5">
				<div class="grid gap-1.5">
					<span class="text-sm font-semibold text-ink">{m.status_seo_preview()}</span>
					<!-- A search result as engines draw it: the blue title, the address, the description. -->
					<div class="rounded-lg border border-line bg-surface p-4" role="group" aria-label={m.status_seo_preview_aria()}>
						<p class="truncate text-[0.75rem] text-ink-2">{breadcrumb(shown.canonical || url) || 'example.com'}</p>
						<p class="mt-0.5 text-lg leading-snug text-info">{truncate(shown.title, TITLE_TARGET)}</p>
						<p class="mt-0.5 text-sm text-ink-2">{truncate(shown.description, DESCRIPTION_TARGET)}</p>
					</div>
					{#if !seo.index}
						<p class="text-[0.8125rem] text-ink-2">{m.status_seo_preview_hidden()}</p>
					{/if}
				</div>

				<div class="grid gap-2 rounded-lg border border-line p-4">
					<div class="flex items-center gap-2 text-sm font-semibold text-ink">
						<Sparkles class="size-4" aria-hidden="true" />
						{m.status_seo_ai_title()}
					</div>
					<p class="text-[0.8125rem] text-ink-2">{m.status_seo_ai_help()}</p>
					<div class="flex flex-wrap gap-2">
						<Button type="button" variant="ghost" onclick={copyPrompt} {disabled}>
							{#if copied}
								<Check class="size-4" aria-hidden="true" />
								{m.status_seo_ai_copied()}
							{:else}
								<ClipboardCopy class="size-4" aria-hidden="true" />
								{m.status_seo_ai_copy()}
							{/if}
						</Button>
						<Button type="button" variant="ghost" onclick={() => (pasteOpen = !pasteOpen)} {disabled}>
							<ClipboardPaste class="size-4" aria-hidden="true" />
							{m.status_seo_ai_paste()}
						</Button>
					</div>
					{#if copyFailed}
						<p class="text-[0.8125rem] text-warning" role="alert">{m.status_seo_ai_copy_failed()}</p>
					{/if}
					{#if pasteOpen}
						<label for="{idPrefix}-seo-paste" class="text-sm font-semibold text-ink">{m.status_seo_paste_label()}</label>
						<textarea
							id="{idPrefix}-seo-paste"
							class="input min-h-28 font-mono text-[0.8125rem]"
							rows="5"
							bind:value={pasted}
							placeholder={'{"title": "…", "description": "…"}'}
							spellcheck="false"
							{disabled}
						></textarea>
						<div>
							<Button type="button" variant="primary" onclick={applyPasted} {disabled}>{m.status_seo_paste_apply()}</Button>
						</div>
						{#if pasteError}
							<p class="text-[0.8125rem] text-warning" role="alert">{pasteError}</p>
						{/if}
						{#if applied}
							<p class="text-[0.8125rem] text-ink-2" role="status">{m.status_seo_paste_done()}</p>
						{/if}
						{#each pasteNotes as note (note)}
							<p class="text-[0.8125rem] text-warning">{note}</p>
						{/each}
					{/if}
				</div>
			</div>
		</div>
	</div>
</Panel>
