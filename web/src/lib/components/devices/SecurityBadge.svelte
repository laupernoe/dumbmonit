<script lang="ts">
	/**
	 * A device's security score as a small badge: shield, the letter and, when
	 * there is room, the score — the letter is text, so the grade never rests
	 * on the tone alone. Always interactive: a link to the device's report
	 * (device list), or a button that opens it in place (device page). The
	 * hit area is at least 40 px tall even when the plate itself is smaller.
	 */
	import { ShieldCheck } from 'lucide-svelte';
	import type { SecurityGrade } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { GRADE_TONE } from './securityFormat';

	interface Props {
		grade: SecurityGrade | null;
		score: number | null;
		/** Link to the report; without it the badge is a button calling `onclick`. */
		href?: string;
		onclick?: () => void;
		/** Button mode: is the report it controls open? */
		expanded?: boolean;
		/** Id of the report the button controls. */
		controls?: string;
		/** `md` spells "Security" out and shows the score; `sm` keeps to the letter. */
		size?: 'sm' | 'md';
		class?: string;
	}

	let { grade, score, href, onclick, expanded, controls, size = 'sm', class: className = '' }: Props = $props();

	const TONE_CLASS = {
		signal: 'bg-signal-soft text-signal-ink border-signal/30',
		advisory: 'bg-advisory-soft text-advisory-ink border-advisory/35',
		warning: 'bg-warning-soft text-warning-ink border-warning/35',
		ghost: 'bg-ghost text-ink-2 border-line'
	} as const;

	const tone = $derived(grade ? (GRADE_TONE[grade] as keyof typeof TONE_CLASS) : 'ghost');
	const label = $derived(
		grade !== null && score !== null ? m.devices_security_badge_aria({ grade, score }) : m.devices_security_badge_unrated_aria()
	);
	const wrap = $derived(`group inline-flex min-h-10 shrink-0 items-center rounded-[var(--radius-plate)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal ${className}`);
</script>

{#snippet plate()}
	<span
		class={`inline-flex items-center gap-1.5 rounded-[var(--radius-plate)] border font-semibold leading-none transition-colors group-hover:border-line-strong ${size === 'sm' ? 'h-6 px-1.5 text-[0.75rem]' : 'h-7 px-2.5 text-[0.8125rem]'} ${TONE_CLASS[tone]}`}
	>
		<ShieldCheck class="size-3.5 shrink-0" aria-hidden="true" />
		{#if size === 'md'}
			<span>{grade !== null ? m.devices_security_badge_md({ grade }) : m.devices_security_badge_unrated()}</span>
			{#if score !== null}<span class="tnum font-normal opacity-80">{score}/100</span>{/if}
		{:else}
			<span>{grade ?? '?'}</span>
		{/if}
	</span>
{/snippet}

{#if href}
	<a {href} class={wrap} aria-label={label} title={label}>{@render plate()}</a>
{:else}
	<button type="button" class={wrap} aria-label={label} title={label} aria-expanded={expanded} aria-controls={controls} {onclick}>
		{@render plate()}
	</button>
{/if}
