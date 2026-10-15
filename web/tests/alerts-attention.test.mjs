// Info alerts are not problems: nav badge and wall count only warnings and criticals. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { attentionCount, countsAsProblem } from '../src/lib/alerts-attention.ts';

const alert = (over) => ({ effective_phase: 'firing', acked: false, severity: 'warning', ...over });

test('warning and critical firing alerts count', () => {
	assert.equal(countsAsProblem(alert({})), true);
	assert.equal(countsAsProblem(alert({ severity: 'critical' })), true);
});

test('info alerts never count', () => {
	assert.equal(countsAsProblem(alert({ severity: 'info' })), false);
});

test('acknowledged or suppressed alerts do not count', () => {
	assert.equal(countsAsProblem(alert({ acked: true })), false);
	assert.equal(countsAsProblem(alert({ effective_phase: 'suppressed' })), false);
});

test('the wall total leaves info out', () => {
	assert.equal(attentionCount({ warnings: 0, advisories: 1, unreachable: 1, misconfigured: 0 }), 2);
});
