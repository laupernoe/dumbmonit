// Data & retention helpers and log-search periods. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { diffUpdate, diskShare, formatBytes, formFrom, lowersRawRetention, parseDays, periodRange } from '../src/lib/components/settings/storage.ts';

const view = {
	metrics: { embedded: true, retention_days: 90, retention_source: 'default', running: true },
	rollup: { enabled: false, retention_days: 730 },
	logs_store: { enabled: false, retention_days: 30 }
};

test('sizes are human, unknown is a dash', () => {
	assert.equal(formatBytes(0), '0 B');
	assert.equal(formatBytes(1536), '1.5 kB');
	assert.equal(formatBytes(5 * 1024 ** 3), '5.0 GB');
	assert.equal(formatBytes(null), '—');
});

test('disk share is clamped and 0 without a total', () => {
	assert.equal(diskShare(50, 200), 25);
	assert.equal(diskShare(500, 200), 100);
	assert.equal(diskShare(5, null), 0);
	assert.equal(diskShare(null, 100), 0);
});

test('days must be whole numbers within limits', () => {
	assert.equal(parseDays('30', 7, 3650), 30);
	assert.equal(parseDays(6, 7, 3650), null);
	assert.equal(parseDays(1.5, 1, 10), null);
	assert.equal(parseDays('', 1, 10), null);
});

test('only changed fields are sent; external VM never sends raw retention', () => {
	const form = formFrom(view);
	assert.deepEqual(diffUpdate(view, form), {});
	assert.deepEqual(diffUpdate(view, { ...form, metricsDays: 30, rollupEnabled: true }), {
		metrics_retention_days: 30,
		rollup_enabled: true
	});
	const external = { ...view, metrics: { ...view.metrics, embedded: false } };
	assert.deepEqual(diffUpdate(external, { ...form, metricsDays: 30 }), {});
	assert.equal(lowersRawRetention(view, { metrics_retention_days: 30 }), true);
	assert.equal(lowersRawRetention(view, { metrics_retention_days: 365 }), false);
});

test('search periods', () => {
	assert.deepEqual(periodRange('1h', 10_000_000), { start: 10_000_000 - 3_600_000, end: 10_000_000 });
	assert.equal(periodRange('custom', 0), null);
	assert.equal(periodRange('custom', 0, { start: '2026-10-02T10:00', end: '2026-10-02T09:00' }), null);
	const r = periodRange('custom', 0, { start: '2026-10-02T10:00', end: '2026-10-02T11:00' });
	assert.equal(r.end - r.start, 3_600_000);
});
