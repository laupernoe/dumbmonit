<script lang="ts">
	/**
	 * Deleting an agent's monitor does not remove the agent from the machine: it
	 * keeps running and asks to enrol again. This shows how to take it off first
	 * (commands matched to the machine's system), then deletes the monitor.
	 */
	import { onMount } from 'svelte';
	import { getAgentHost } from '#lib/api/index.js';
	import type { Target } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Button, CopyBlock, Panel } from '#lib/ui/index.js';

	interface Props {
		target: Target;
		deleting: boolean;
		ondelete: () => void;
		oncancel: () => void;
	}
	let { target, deleting, ondelete, oncancel }: Props = $props();

	let os = $state('');
	onMount(() => {
		const controller = new AbortController();
		getAgentHost(target.id, controller.signal)
			.then((host) => (os = (host?.os ?? '').toLowerCase()))
			.catch(() => {});
		return () => controller.abort();
	});

	const commands = $derived.by(() => {
		if (os.includes('win')) {
			return {
				sudo: false,
				list: ['Stop-Service DumbMonitAgent -Force; sc.exe delete DumbMonitAgent; Remove-Item -Recurse -Force "$env:ProgramFiles\\DumbMonit", "$env:ProgramData\\DumbMonit"']
			};
		}
		if (os.includes('freebsd')) {
			return {
				sudo: true,
				list: ['service dumbmonit_agent onestop; sysrc -x dumbmonit_agent_enable; rm -rf /usr/local/etc/rc.d/dumbmonit_agent /usr/local/lib/dumbmonit-agent /usr/local/etc/dumbmonit']
			};
		}
		if (os.includes('darwin') || os.includes('mac')) {
			return {
				sudo: true,
				list: ['launchctl bootout system/com.dumbmonit.agent; rm -rf /Library/LaunchDaemons/com.dumbmonit.agent.plist /usr/local/lib/dumbmonit-agent /usr/local/etc/dumbmonit']
			};
		}
		return {
			sudo: true,
			list: [
				'systemctl disable --now dumbmonit-agent && rm -rf /etc/systemd/system/dumbmonit-agent.service /usr/local/bin/dumbmonit-agent /usr/local/lib/dumbmonit-agent /etc/dumbmonit && systemctl daemon-reload',
				'docker rm -f dumbmonit-agent && docker volume rm dumbmonit-agent'
			]
		};
	});
</script>

<Panel title={m.devices_agent_remove_title()} description={m.devices_agent_remove_text()} class="mt-4">
	<div class="grid gap-3">
		{#each commands.list as cmd (cmd)}
			<CopyBlock value={cmd} sudo={commands.sudo} />
		{/each}
		<p class="text-sm text-ink-2">{m.devices_agent_remove_docker()}</p>
		<div class="flex flex-wrap items-center gap-3">
			<Button variant="danger" loading={deleting} onclick={ondelete}>{m.devices_agent_remove_delete()}</Button>
			<Button variant="ghost" onclick={oncancel}>{m.devices_agent_remove_cancel()}</Button>
		</div>
	</div>
</Panel>
