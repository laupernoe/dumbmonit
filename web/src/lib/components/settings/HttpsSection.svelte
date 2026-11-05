<script lang="ts">
	import { withBase } from '#lib/paths.js';
	/**
	 * Settings → System → HTTPS and encryption: the certificate the web port
	 * serves (provided or self-signed, its fingerprint to compare with the
	 * browser's; the web port only serves HTTPS), and every place where data
	 * still travels in clear or without certificate checks, with a link to
	 * where it is fixed. Administrators only (`GET /api/security/https`).
	 */
	import { Download, Lock, ShieldCheck } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getHttps, HTTPS_CERTIFICATE_URL } from '#lib/api/https.js';
	import type { HttpsFinding, HttpsView } from '#lib/api/types.js';
	import { formatDateTime } from '#lib/format.js';
	import { DocLink, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';

	let data = $state<HttpsView | null>(null);
	let loadError = $state<unknown>(null);

	$effect(() => {
		const controller = new AbortController();
		getHttps(controller.signal)
			.then((view) => (data = view))
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				loadError = cause;
			});
		return () => controller.abort();
	});

	/** One sentence per finding code; an unknown code shows as it is. */
	function describe(finding: HttpsFinding): string {
		const subject = finding.subject;
		switch (finding.code) {
			case 'https_unavailable':
				return m.settings_https_finding_https_unavailable();
			case 'self_signed':
				return m.settings_https_finding_self_signed();
			case 'victoria_http':
				return m.settings_https_finding_victoria_http({ subject });
			case 'oidc_http':
				return m.settings_https_finding_oidc_http();
			case 'ldap_plain':
				return m.settings_https_finding_ldap_plain();
			case 'ldap_unverified':
				return m.settings_https_finding_ldap_unverified();
			case 'saml_http':
				return m.settings_https_finding_saml_http();
			case 'tls_unverified':
				return m.settings_https_finding_tls_unverified({ subject });
			case 'snmp_community':
				return m.settings_https_finding_snmp_community({ subject });
			case 'snmp_no_privacy':
				return m.settings_https_finding_snmp_no_privacy({ subject });
			case 'credentials_over_http':
				return m.settings_https_finding_credentials_over_http({ subject });
			case 'smtp_plain':
				return m.settings_https_finding_smtp_plain({ subject });
			case 'channel_http':
				return m.settings_https_finding_channel_http({ subject });
			case 'log_plain':
				return m.settings_https_finding_log_plain({ subject });
			case 'agents_legacy_allowed':
				return m.settings_https_finding_agents_legacy({ count: subject });
			case 'unsigned_updates':
				return m.settings_https_finding_unsigned_updates();
			case 'local_backup_with_key':
				return m.settings_https_finding_local_backup({ subject });
			default:
				return subject ? `${finding.code}: ${subject}` : finding.code;
		}
	}

	const warnings = $derived(data?.findings.filter((f) => f.severity === 'warning') ?? []);
	const infos = $derived(data?.findings.filter((f) => f.severity !== 'warning') ?? []);
</script>

{#snippet findingList(list: HttpsFinding[], tone: 'warning' | 'info')}
	<ul class="grid gap-2">
		{#each list as finding, index (`${finding.code}-${finding.subject}-${index}`)}
			<li class="flex flex-wrap items-start gap-x-3 gap-y-1 rounded-[var(--radius-card)] border border-line px-3 py-2">
				<Plate {tone} label={tone === 'warning' ? m.settings_https_severity_warning() : m.settings_https_severity_info()} />
				<p class="min-w-0 flex-1 text-sm break-words text-ink">{describe(finding)}</p>
				{#if finding.link}
					<a href={withBase(finding.link)} class="inline-flex min-h-10 items-center text-sm font-semibold text-signal-ink hover:underline">{m.settings_https_fix()}</a>
				{/if}
			</li>
		{/each}
	</ul>
{/snippet}

<Panel id="https" title={m.settings_https_title()} description={m.settings_https_description()}>
	{#if loadError}
		<ErrorNotice error={loadError} title={m.settings_https_load_error()} />
	{:else if !data}
		<Skeleton class="h-32" />
	{:else}
		<div class="grid gap-5">
			<p class="flex items-center gap-2 text-sm text-ink">
				<Lock class="size-4 shrink-0 text-signal-ink" aria-hidden="true" />
				{m.settings_https_only()}
			</p>

			{#if data.certificate}
				<div class="grid gap-2 rounded-[var(--radius-card)] border border-line bg-surface-2 px-3 py-3">
					<div class="flex flex-wrap items-center gap-2">
						<ShieldCheck class="size-4 shrink-0 text-ink-2" aria-hidden="true" />
						<p class="text-sm font-semibold text-ink">{m.settings_https_certificate()}</p>
						<Plate
							tone={data.certificate.source === 'provided' ? 'signal' : 'advisory'}
							label={data.certificate.source === 'provided' ? m.settings_https_source_provided() : m.settings_https_source_self_signed()}
						/>
					</div>
					<dl class="grid gap-x-4 gap-y-1 text-sm sm:grid-cols-[auto_1fr]">
						<dt class="text-ink-2">{m.settings_https_fingerprint()}</dt>
						<dd class="font-mono text-[0.8125rem] break-all text-ink">{data.certificate.fingerprint_sha256}</dd>
						{#if data.certificate.not_after}
							<dt class="text-ink-2">{m.settings_https_expires()}</dt>
							<dd class="tnum text-ink">{formatDateTime(data.certificate.not_after)}</dd>
						{/if}
						{#if data.certificate.names.length > 0}
							<dt class="text-ink-2">{m.settings_https_names()}</dt>
							<dd class="font-mono text-[0.8125rem] break-all text-ink">{data.certificate.names.join(', ')}</dd>
						{/if}
						{#if data.certificate.source === 'provided'}
							<dt class="text-ink-2">{m.settings_https_issuer()}</dt>
							<dd class="break-words text-ink">{data.certificate.issuer}</dd>
						{/if}
					</dl>
					<a href={withBase(HTTPS_CERTIFICATE_URL)} download class="inline-flex min-h-10 items-center gap-1.5 justify-self-start text-sm font-semibold text-signal-ink hover:underline">
						<Download class="size-4" aria-hidden="true" />
						{m.settings_https_download()}
					</a>
				</div>
			{:else}
				<p class="max-w-prose text-sm text-ink">{m.settings_https_unavailable()}</p>
			{/if}
			{#if data.error}
				<ErrorNotice error={new Error(data.error)} title={m.settings_https_reload_error()} />
			{/if}

			<p class="max-w-prose text-sm text-ink-2">{m.settings_https_hsts({ value: data.hsts })}</p>

			<div class="grid gap-2">
				<h4 class="text-sm font-semibold text-ink">{m.settings_https_findings_title()}</h4>
				{#if warnings.length === 0 && infos.length === 0}
					<p class="text-sm text-ink-2">{m.settings_https_findings_none()}</p>
				{:else}
					{#if warnings.length > 0}{@render findingList(warnings, 'warning')}{/if}
					{#if infos.length > 0}{@render findingList(infos, 'info')}{/if}
				{/if}
			</div>
		</div>
	{/if}
	<DocLink page="install/https/" class="mt-3" />
</Panel>
