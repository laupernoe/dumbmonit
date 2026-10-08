<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * SHA-256 checksums of the agent binaries this server ships, shown next to the
	 * install command. The installer verifies the download against them on its
	 * own; showing them here lets a careful admin compare by eye or by hand
	 * (`sha256sum dumbmonit-agent-linux-x86_64`) when the server is reached over
	 * plain HTTP. Nothing is shown when the image carries no binaries.
	 */
	import { fetchAgentChecksums, type AgentChecksum } from '#lib/api/agent_files.js';

	let checksums = $state<AgentChecksum[]>([]);

	$effect(() => {
		let cancelled = false;
		void fetchAgentChecksums().then((found) => {
			if (!cancelled) checksums = found.filter((entry) => entry.sha256 !== null);
		});
		return () => {
			cancelled = true;
		};
	});
</script>

{#if checksums.length > 0}
	<details class="group rounded-lg border border-line bg-canvas-deep px-3 py-2 text-sm">
		<summary class="cursor-pointer select-none text-ink-2">
			{m.deviceform_checksums_summary()}
		</summary>
		<dl class="mt-2 grid gap-1.5">
			{#each checksums as entry (entry.file)}
				<div class="grid gap-0.5">
					<dt class="font-semibold text-ink">{entry.file}</dt>
					<dd class="break-all font-mono text-[0.75rem] leading-relaxed text-ink-2">{entry.sha256}</dd>
				</div>
			{/each}
		</dl>
	</details>
{/if}
