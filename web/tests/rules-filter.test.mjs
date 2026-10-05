// Alerts rules: which collector kind a built-in rule is about, and whether
// it belongs in the default ("relevant") view. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { ruleKinds, isRelevant } from '../src/lib/components/alerts/rules-filter.ts';

const KNOWN_KINDS = ['redis', 'unifi', 'proxmox', 'npm', 'dns', 'domain', 'nut', 'kubernetes', 'synology'];

function rule(overrides) {
	return {
		id: 1,
		uid: 'some_rule',
		name: 'Some rule',
		description: '',
		kind: 'threshold',
		query: 'dumbmonit_up',
		operator: '>',
		threshold: 0,
		clear_threshold: null,
		for_secs: 0,
		severity: 'warning',
		selector: 'all',
		channels: [],
		params: {},
		unit: '',
		repeat_secs: null,
		escalate_after_secs: null,
		enabled: true,
		builtin: true,
		overrides: [],
		...overrides
	};
}

test('a universal rule (no kind token at all) has no kinds and is always relevant', () => {
	const hostDown = rule({
		uid: 'host_down',
		query: 'time() - tlast_over_time(dumbmonit_up[7d])'
	});
	assert.deepEqual(ruleKinds(hostDown, KNOWN_KINDS), []);
	assert.equal(isRelevant(hostDown, new Set(), KNOWN_KINDS), true);
	assert.equal(isRelevant(hostDown, new Set(['redis']), KNOWN_KINDS), true);
});

test('a kind-specific rule names exactly that kind', () => {
	const redis = rule({
		uid: 'redis_memory_near_limit',
		query: 'dumbmonit_redis_memory_used_percent / dumbmonit_redis_memory_max_bytes * 100'
	});
	assert.deepEqual(ruleKinds(redis, KNOWN_KINDS), ['redis']);

	const unifi = rule({ uid: 'unifi_alarms', query: 'dumbmonit_unifi_alarms' });
	assert.deepEqual(ruleKinds(unifi, KNOWN_KINDS), ['unifi']);
});

test('a kind-specific rule is relevant only when that kind is owned', () => {
	const redis = rule({
		uid: 'redis_memory_near_limit',
		query: 'dumbmonit_redis_memory_used_percent / dumbmonit_redis_memory_max_bytes * 100'
	});
	assert.equal(isRelevant(redis, new Set(), KNOWN_KINDS), false);
	assert.equal(isRelevant(redis, new Set(['unifi']), KNOWN_KINDS), false);
	assert.equal(isRelevant(redis, new Set(['redis', 'unifi']), KNOWN_KINDS), true);
});

test('token matching respects boundaries: "dns" never matches inside "domain" and vice versa', () => {
	const domainRule = rule({ uid: 'domain_expiry_near', query: 'dumbmonit_domain_expiry_days < 14' });
	assert.deepEqual(ruleKinds(domainRule, KNOWN_KINDS), ['domain']);

	const dnsRule = rule({ uid: 'dns_lookup_failed', query: 'dumbmonit_dns_up == 0' });
	assert.deepEqual(ruleKinds(dnsRule, KNOWN_KINDS), ['dns']);

	// A short kind name must not match as a substring of another token
	// ("np" inside "npm").
	assert.deepEqual(ruleKinds(rule({ query: 'dumbmonit_npm_host_online == 0' }), ['np', 'npm']), ['npm']);
});

test('an "or" across a kind-specific metric and a fully generic one is universal, not kind-specific', () => {
	// Mirrors the real "High CPU" / "Disk almost full" rules: they union a
	// Proxmox metric with a generic one so they still fire for everyone.
	const cpuHigh = rule({
		uid: 'cpu_high',
		query:
			'avg by (target, host) (dumbmonit_cpu_load_percent or dumbmonit_proxmox_node_cpu_percent or dumbmonit_cpu_usage_percent)'
	});
	assert.deepEqual(ruleKinds(cpuHigh, KNOWN_KINDS), []);
	assert.equal(isRelevant(cpuHigh, new Set(), KNOWN_KINDS), true);
});

test('an "or" across several kind-specific metrics, none generic, names all of them', () => {
	const combined = rule({ query: 'dumbmonit_proxmox_node_cpu_percent or dumbmonit_unifi_cpu_percent' });
	assert.deepEqual(ruleKinds(combined, KNOWN_KINDS).sort(), ['proxmox', 'unifi']);
	assert.equal(isRelevant(combined, new Set(['redis']), KNOWN_KINDS), false);
	assert.equal(isRelevant(combined, new Set(['unifi']), KNOWN_KINDS), true);
});

test('a kind whose shipped rules abbreviate its metric prefix is still recognised', () => {
	// Real cases: Kubernetes rules use `dumbmonit_k8s_…`, Synology's Active
	// Backup for Business rules use `dumbmonit_abb_…`.
	const k8s = rule({ uid: 'k8s_node_not_ready', query: 'dumbmonit_k8s_node_ready == bool 0' });
	assert.deepEqual(ruleKinds(k8s, KNOWN_KINDS), ['kubernetes']);
	assert.equal(isRelevant(k8s, new Set(), KNOWN_KINDS), false);
	assert.equal(isRelevant(k8s, new Set(['kubernetes']), KNOWN_KINDS), true);

	const abb = rule({ uid: 'abb_task_failed', query: 'dumbmonit_abb_task_last_status == bool 0' });
	assert.deepEqual(ruleKinds(abb, KNOWN_KINDS), ['synology']);
	assert.equal(isRelevant(abb, new Set(['synology']), KNOWN_KINDS), true);
});

test('a disabled or non-builtin rule is always relevant, regardless of kind', () => {
	const disabledRedis = rule({
		query: 'dumbmonit_redis_memory_used_percent',
		enabled: false
	});
	assert.equal(isRelevant(disabledRedis, new Set(), KNOWN_KINDS), true);

	const customRedis = rule({
		query: 'dumbmonit_redis_memory_used_percent',
		builtin: false
	});
	assert.equal(isRelevant(customRedis, new Set(), KNOWN_KINDS), true);
});
