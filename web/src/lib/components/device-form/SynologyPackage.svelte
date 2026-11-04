<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/** Download and install steps of the DSM 7 agent package, above the token enrolment. */
	import { Download } from 'lucide-svelte';
	import { getHealth } from '#lib/api/index.js';
	import { DocLink } from '#lib/ui/index.js';
	import { SPK_ARCHES, spkUrl } from './synology-package.js';

	let version = $state<string | null>(null);
	$effect(() => {
		const controller = new AbortController();
		getHealth(controller.signal)
			.then((health) => (version = health.version))
			.catch(() => {});
		return () => controller.abort();
	});

	const button =
		'inline-flex min-h-10 items-center gap-2 rounded-lg border border-line bg-surface px-3.5 text-sm font-semibold text-ink transition-colors hover:border-line-strong focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal';
</script>

<div class="mb-5 grid gap-3 rounded-[var(--radius-card)] border border-line bg-surface-2 px-4 py-3.5">
	<p class="text-sm font-semibold text-ink">{m.deviceform_synology_dl_title()}</p>
	{#if version}
		<div class="flex flex-wrap gap-2">
			{#each SPK_ARCHES as arch (arch)}
				<a class={button} href={spkUrl(version, arch)} rel="noopener">
					<Download class="size-4" aria-hidden="true" />
					{m.deviceform_synology_dl_arch({ arch })}
				</a>
			{/each}
		</div>
	{/if}
	<DocLink page="devices/synology-agent/" />
</div>
