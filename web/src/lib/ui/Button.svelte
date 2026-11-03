<script lang="ts">
	/**
	 * The one button. `primary` is the solid one (one per view, the thing to do
	 * next) and answers a click with the green spark burst; `secondary` is an
	 * outlined faceplate; `ghost` is text; `danger` is outlined red and asks
	 * for a `confirm` word when the action is final.
	 *
	 * Success feedback: when `onclick` returns a promise that resolves (and not
	 * to `false`), the button turns signal-green with a tick for ~1.2 s, at the
	 * same size, then reverts. A rejection or `false` stays silent. `feedback`
	 * does the same for a synchronous handler, `success` (rising edge) lets the
	 * page trigger it. Announced to screen readers; the tick means it is not
	 * colour alone.
	 */
	import type { Snippet } from 'svelte';
	import { onDestroy } from 'svelte';
	import { Check, Loader2 } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { sparkle } from './effects/spark';
	import { FEEDBACK_MS, outcome } from './feedback';

	interface Props {
		children: Snippet;
		variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
		size?: 'sm' | 'md' | 'lg';
		type?: 'button' | 'submit';
		href?: string;
		disabled?: boolean;
		loading?: boolean;
		class?: string;
		onclick?: (e: MouseEvent) => unknown;
		/** Confirm a synchronous handler too (a promise is always awaited). */
		feedback?: boolean;
		/** Set true by the page when the action succeeded: shows the confirmation once. */
		success?: boolean;
		[key: string]: unknown;
	}

	let {
		children,
		variant = 'secondary',
		size = 'md',
		type = 'button',
		href,
		disabled = false,
		loading = false,
		class: className = '',
		onclick,
		feedback = false,
		success = false,
		...rest
	}: Props = $props();

	let done = $state(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	function confirmed() {
		done = true;
		clearTimeout(timer);
		timer = setTimeout(() => (done = false), FEEDBACK_MS);
	}
	onDestroy(() => clearTimeout(timer));

	async function click(e: MouseEvent) {
		const result = onclick?.(e);
		if (result === undefined || result === null) {
			if (feedback) confirmed();
			return;
		}
		if (typeof result === 'object' && 'then' in result ? await outcome(result) : result !== false) confirmed();
	}

	let seen = false;
	$effect(() => {
		if (success && !seen) confirmed();
		seen = success;
	});

	const base =
		'relative inline-flex select-none items-center justify-center gap-2 whitespace-nowrap rounded-lg font-semibold transition-[transform,box-shadow,background-color,border-color,color] duration-150 ease-out-expo max-sm:duration-100 disabled:cursor-not-allowed disabled:opacity-50 active:translate-y-px';
	const sizes = {
		sm: 'h-8 max-sm:h-10 px-3 text-[0.8125rem]',
		md: 'h-10 px-4 text-sm',
		lg: 'h-12 px-6 text-base'
	};
	const variants = {
		primary:
			'bg-primary text-on-primary shadow-lift hover:brightness-110 hover:shadow-float overflow-hidden before:absolute before:inset-0 before:-translate-x-full before:bg-gradient-to-r before:from-transparent before:via-white/30 before:to-transparent before:transition-transform before:duration-700 hover:before:translate-x-full',
		secondary:
			'border border-line-strong bg-surface text-ink shadow-lift hover:border-ink-3 hover:shadow-float',
		ghost: 'text-ink-2 hover:bg-surface-2 hover:text-ink',
		danger:
			'border border-warning/50 bg-surface text-warning-ink hover:bg-warning-soft hover:border-warning'
	};

	const classes = $derived(`${base} ${sizes[size]} ${variants[variant]} ${className}`);
	const doneClasses = '!border-signal !bg-signal !text-on-signal';
</script>

{#if href && !disabled}
	<a {href} class={classes} {onclick} {...rest} use:sparkle={{ enabled: variant === 'primary' }}>
		{@render children()}
	</a>
{:else}
	<button
		{type}
		class={`${classes} ${done ? doneClasses : ''}`}
		disabled={disabled || loading}
		aria-busy={loading}
		onclick={click}
		{...rest}
		use:sparkle={{ enabled: variant === 'primary' }}
	>
		<!-- The label keeps its box (hidden, not removed) so the tick never changes the width. -->
		<span class={`inline-flex items-center justify-center gap-[inherit] transition-opacity duration-150 ${done ? 'opacity-0' : ''}`}>
			{#if loading}
				<Loader2 class="size-4 animate-spin" aria-hidden="true" />
			{/if}
			{@render children()}
		</span>
		{#if done}
			<span class="state-in pointer-events-none absolute inset-0 flex items-center justify-center" aria-hidden="true">
				<Check class="check-draw size-4" />
			</span>
		{/if}
		<span class="sr-only" role="status" aria-live="polite">{done ? m.app_ui_done() : ''}</span>
	</button>
{/if}
