/** Where the DSM 7 agent package is published, and the docs page that explains it. */

export const SPK_ARCHES = ['x86_64', 'aarch64'] as const;
export type SpkArch = (typeof SPK_ARCHES)[number];


/** Release asset of the package: attached to each GitHub release. */
export function spkUrl(version: string, arch: SpkArch): string {
	const v = version.replace(/^v/, '');
	return `https://github.com/laupernoe/dumbmonit/releases/download/v${v}/dumbmonit-agent-${v}-${arch}.spk`;
}

/** Link into the add flow, on the package side of the Synology choice. */
export const SPK_FLOW_HREF = '/targets/new?kind=synology&mode=agent';
