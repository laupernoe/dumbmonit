<script lang="ts">
	/**
	 * Slim strip above the navigation in the public demo: what this is, where to
	 * get it, and the way back into the tour.
	 */
	import { BookOpen, GitBranch, Compass } from 'lucide-svelte';
	import { demo } from '#lib/stores/demo.svelte.js';

	const INSTALL_URL = 'https://dumbmonit.readthedocs.io/en/latest/install/docker/';
	const GITHUB_URL = 'https://github.com/laupernoe/dumbmonit';

	const link =
		'inline-flex h-7 items-center gap-1.5 rounded-md px-2 font-semibold text-ink-2 transition-colors hover:bg-surface-2 hover:text-ink';
</script>

<div class="border-b border-line bg-advisory-soft text-[0.8125rem]">
	<div class="mx-auto flex max-w-7xl flex-wrap items-center gap-x-3 gap-y-1 px-4 py-1.5 sm:px-6">
		<p class="flex min-w-0 items-center gap-2 text-ink">
			<span class="inline-flex items-center gap-1.5 rounded-full border border-advisory/40 px-2 py-0.5 text-[0.6875rem] font-bold tracking-wide text-advisory-ink uppercase">
				<span class="size-1.5 rounded-full bg-advisory" aria-hidden="true"></span>
				Live demo
			</span>
			<span class="truncate">Read-only, fictional data</span>
		</p>
		<nav aria-label="Demo" class="ml-auto flex items-center gap-1">
			<button type="button" class={link} onclick={() => demo.openTour()} data-tour-trigger>
				<Compass class="size-3.5" aria-hidden="true" />
				Take the tour
			</button>
			<a class={link} href={INSTALL_URL} target="_blank" rel="noopener">
				<BookOpen class="size-3.5" aria-hidden="true" />
				Install it
			</a>
			<a class={link} href={GITHUB_URL} target="_blank" rel="noopener">
				<GitBranch class="size-3.5" aria-hidden="true" />
				GitHub
			</a>
		</nav>
	</div>
</div>
