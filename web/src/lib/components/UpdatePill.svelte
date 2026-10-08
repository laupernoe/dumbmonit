<script lang="ts">
	/**
	 * "Update available: vX", a discreet pill above the version tag that opens
	 * Settings → About, where the guided update lives. Silent when up to date,
	 * when the check is off, when signed out, or when anything fails: it must
	 * never get in the way. Mounted by `VersionTag`.
	 */
	import { ArrowUpCircle } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getUpdate } from '#lib/api/update.js';

	let latest = $state<string | null>(null);
	$effect(() => {
		const controller = new AbortController();
		getUpdate(controller.signal, true)
			.then((info) => (latest = info.update_available ? info.latest : null))
			.catch(() => {});
		return () => controller.abort();
	});
</script>

{#if latest}
	<a
		href="/settings#about"
		class="fixed right-3 bottom-9 z-20 hidden items-center gap-1 rounded-full border border-line bg-surface px-2.5 py-1 text-[0.75rem] font-medium text-advisory-ink shadow-sm transition-colors hover:border-line-strong sm:inline-flex"
		title={m.misc_update_title()}
	>
		<ArrowUpCircle class="size-3.5" aria-hidden="true" />
		{m.misc_update_available({ version: latest })}
	</a>
{/if}
