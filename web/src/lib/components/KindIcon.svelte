<script lang="ts">
	/**
	 * The icon of a collector kind: the product's own mark when there is one
	 * (`brand-icons.ts`, generated from Simple Icons), drawn in the current
	 * text colour like every other icon and inset to the optical size of a Lucide stroke icon, else the generic Lucide icon of
	 * `kinds.ts` — so a kind this build does not know still gets one.
	 * `platform` (a `website` monitor's detected platform) takes precedence
	 * when it has a mark. Always decorative: the kind's name sits next to it.
	 */
	import { kindBrand, platformBrand } from '#lib/brand-icons.js';
	import { kindIcon } from '#lib/components/device-form/kinds.js';

	interface Props {
		kind: string;
		platform?: string | null;
		class?: string;
		/**
		 * Pixel size, written as `width`/`height` attributes. Needed inside an SVG
		 * (the topology map): a nested `<svg>` ignores a CSS size class and fills
		 * its parent, so the mark would come out gigantic.
		 */
		size?: number;
	}

	let { kind, platform = null, class: className = 'size-4', size }: Props = $props();

	// Synology's mark is a very wide wordmark: inset into a square it is unreadable at icon size.
	const WORDMARKS = new Set(['synology']);
	const brand = $derived(platformBrand(platform) ?? (WORDMARKS.has(kind) ? null : kindBrand(kind)));
	const Fallback = $derived(kindIcon(kind));
</script>

{#if brand}
	<svg viewBox="-1.5 -1.5 27 27" width={size} height={size} class={`shrink-0 ${className}`} fill="currentColor" aria-hidden="true" focusable="false">
		<path d={brand.path} />
	</svg>
{:else}
	<Fallback class={`shrink-0 ${className}`} {size} aria-hidden="true" />
{/if}
