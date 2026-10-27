// Jobs x days matrix: days rebuilt from the series the backup collectors store. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	HISTORY_DAYS,
	SOURCES,
	deriveJob,
	groupSeries,
	localDate,
	metricNames
} from '../src/lib/components/devices/backup/history.ts';

const DAY = 86_400;
const HOUR = 3600;
/** 2026-09-30 00:00 UTC. */
const D0 = Date.UTC(2026, 8, 30) / 1000;

/** Samples every hour from `from` to `to` inclusive. */
function every(from, to, value) {
	const out = [];
	for (let t = from; t <= to; t += HOUR) out.push([t, value(t)]);
	return out;
}

test('localDate follows the zone offset', () => {
	assert.equal(localDate(D0 + 23 * HOUR, 0), '2026-09-30');
	assert.equal(localDate(D0 + 23 * HOUR, 120), '2026-10-01');
	assert.equal(localDate(D0 + HOUR, -120), '2026-09-29');
});

test('run age mode (Veeam): one run per day, outcome read once the job is idle', () => {
	const now = D0 + 12 * HOUR;
	// A run at 01:00 on D-2 (ok), D-1 (failed), D0 (warning), each 600 s longer than the last.
	const runs = [
		{ at: D0 - 2 * DAY + HOUR, failed: 0, warning: 0, duration: 600 },
		{ at: D0 - DAY + HOUR, failed: 1, warning: 0, duration: 1200 },
		{ at: D0 + HOUR, failed: 0, warning: 1, duration: 1800 }
	];
	const lastRun = (t) => runs.filter((r) => r.at <= t).at(-1);
	const from = D0 - 2 * DAY + 2 * HOUR;
	const job = {
		key: 'Daily VMs',
		title: 'Daily VMs',
		subtitle: 'VSphereBackup',
		runAge: every(from, now, (t) => t - lastRun(t).at),
		failed: every(from, now, (t) => lastRun(t).failed),
		warning: every(from, now, (t) => lastRun(t).warning),
		running: every(from, now, () => 0),
		duration: every(from, now, (t) => lastRun(t).duration)
	};
	const out = deriveJob(job, { days: 5, nowSeconds: now, offsetMinutes: 0 });
	assert.deepEqual(
		out.days.map((d) => d.state),
		['nodata', 'nodata', 'ok', 'failed', 'warning']
	);
	assert.deepEqual(
		out.days.map((d) => d.date),
		['2026-09-26', '2026-09-27', '2026-09-28', '2026-09-29', '2026-09-30']
	);
	assert.equal(out.days[3].runs.length, 1);
	assert.equal(out.days[3].runs[0].at, D0 - DAY + HOUR);
	assert.equal(out.days[3].runs[0].durationSeconds, 1200);
	assert.equal(out.lastSuccess, D0 + HOUR, 'a warning still backed the data up');
	assert.equal(out.failing, false);
	assert.equal(out.running, false);
	assert.equal(out.durationSeconds, 1800);
	assert.equal(out.bytes, null);
});

test('a run in progress is not judged on the previous result', () => {
	const now = D0 + 13 * HOUR + 1800;
	const start = D0 + 13 * HOUR;
	const from = D0 + 2 * HOUR;
	// Run at 01:00 failed; a new one started at 13:00, the failed flag is still the old result.
	const lastRun = (t) => (t >= start ? start : D0 + HOUR);
	const job = {
		key: 'j',
		title: 'j',
		subtitle: '',
		runAge: every(from, now, (t) => t - lastRun(t)),
		failed: every(from, now, () => 1),
		running: every(from, now, (t) => (t >= start ? 1 : 0))
	};
	const out = deriveJob(job, { days: 3, nowSeconds: now });
	const today = out.days.at(-1);
	assert.deepEqual(
		today.runs.map((r) => r.outcome),
		['failed', 'running']
	);
	assert.equal(today.state, 'failed');
	assert.equal(out.running, true);
	assert.equal(out.lastSuccess, null);
});

test('success age mode (Duplicati): failures are dated by the failed flag', () => {
	const now = D0 + 12 * HOUR;
	const success = D0 - 3 * DAY + HOUR;
	const from = D0 - 3 * DAY + 2 * HOUR;
	const failFrom = D0 - 2 * DAY + 2 * HOUR;
	const job = {
		key: 'Documents',
		title: 'Documents',
		subtitle: '',
		successAge: every(from, now, (t) => t - success),
		failed: every(from, now, (t) => (t >= failFrom ? 1 : 0)),
		bytes: every(from, now, () => 5_000_000)
	};
	const out = deriveJob(job, { days: 5, nowSeconds: now });
	assert.deepEqual(
		out.days.map((d) => d.state),
		['nodata', 'ok', 'failed', 'failed', 'failed']
	);
	assert.equal(out.lastSuccess, success);
	assert.equal(out.failing, true);
	assert.equal(out.bytes, 5_000_000);
	assert.equal(out.days[1].runs[0].bytes, 5_000_000);
});

test('a day with no run and no failure is "none", before any data "nodata"', () => {
	const now = D0 + 12 * HOUR;
	const run = D0 - 2 * DAY + HOUR;
	const from = D0 - 4 * DAY;
	const out = deriveJob(
		{
			key: 'k',
			title: 'k',
			subtitle: '',
			runAge: every(from, now, (t) => (t >= run ? t - run : 0)).filter(([t]) => t >= run)
		},
		{ days: 5, nowSeconds: now }
	);
	// Data begins on D-2 (the first sample is the run itself): nothing earlier is known.
	assert.deepEqual(
		out.days.map((d) => d.state),
		['nodata', 'nodata', 'ok', 'none', 'none']
	);
});

test('the window is thirty days by default', () => {
	const out = deriveJob({ key: 'k', title: 'k', subtitle: '' }, { nowSeconds: D0 + HOUR });
	assert.equal(HISTORY_DAYS, 30);
	assert.equal(out.days.length, 30);
	assert.equal(out.days.at(-1).date, '2026-09-30');
	assert.ok(out.days.every((d) => d.state === 'nodata'));
	assert.equal(out.lastSuccess, null);
});

test('groupSeries gathers the series of one job and maps status codes', () => {
	const abb = SOURCES.abb;
	const labels = (name, extra = {}) => ({ __name__: name, target: 't1', task: 'Office', task_id: '5', ...extra });
	const rows = groupSeries(
		[
			{ metric: labels('dumbmonit_abb_task_last_status', { result: 'failed' }), values: [[D0, '0'], [D0 + HOUR, '2']] },
			{ metric: labels('dumbmonit_abb_task_last_success_seconds'), values: [[D0, '90']] },
			{ metric: labels('dumbmonit_abb_task_enabled'), values: [[D0, '1']] }
		],
		abb
	);
	assert.equal(rows.length, 1);
	assert.equal(rows[0].title, 'Office');
	assert.deepEqual(rows[0].failed, [[D0, 1], [D0 + HOUR, 0]]);
	assert.deepEqual(rows[0].running, [[D0, 0], [D0 + HOUR, 1]]);
	assert.deepEqual(rows[0].successAge, [[D0, 90]]);
});

test('groupSeries leaves out an unreadable Plakar kloset and keeps its sources apart', () => {
	const m = (name, kloset, source) => ({ metric: { __name__: name, kloset, source }, values: [[D0, '1']] });
	const rows = groupSeries(
		[
			m('dumbmonit_backup_last_status', '/srv/k', '*'),
			m('dumbmonit_backup_last_status', '/srv/k', '/photos'),
			m('dumbmonit_backup_last_status', '/srv/k', '/docs')
		],
		SOURCES.plakar
	);
	assert.deepEqual(rows.map((r) => r.title), ['/docs', '/photos']);
	assert.equal(rows[0].subtitle, '/srv/k');
});

test('every source reads metrics that exist, once each in the selector', () => {
	for (const source of Object.values(SOURCES)) {
		const names = metricNames(source);
		assert.ok(names.length > 0);
		assert.equal(new Set(names).size, names.length);
		assert.ok(names.every((n) => n.startsWith('dumbmonit_')));
	}
	assert.ok(metricNames(SOURCES.veeam).includes('dumbmonit_veeam_job_last_duration_seconds'));
});
