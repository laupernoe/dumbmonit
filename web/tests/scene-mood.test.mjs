// Scene mood and pulse derivation. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { toneFrom, diffPulse, createPulseTracker, parseDevOverride, toWallMood } from '../src/lib/scene/mood.ts';

const snap = (down = [], issues = down) => ({ down: new Set(down), issues: new Set(issues) });

test('tone: ok, warn, critical', () => {
	assert.equal(toneFrom({ down: 0, critical: 0, warnings: 0 }), 'ok');
	assert.equal(toneFrom({ down: 0, critical: 0, warnings: 2 }), 'warn');
	assert.equal(toneFrom({ down: 1, critical: 0, warnings: 0 }), 'critical');
	assert.equal(toneFrom({ down: 0, critical: 1, warnings: 1 }), 'critical');
	assert.equal(toWallMood('warn'), 'clouded');
});

test('diff: down, up, resolved, priority', () => {
	assert.equal(diffPulse(snap(), snap(['a'])), 'down');
	assert.equal(diffPulse(snap(['a']), snap()), 'up');
	assert.equal(diffPulse(snap([], ['x']), snap()), 'resolved');
	assert.equal(diffPulse(snap(['a']), snap(['b'])), 'down');
	assert.equal(diffPulse(snap([], ['x']), snap([], ['x'])), null);
});

test('tracker: baseline silent, bursts merge', async () => {
	const out = [];
	const t = createPulseTracker((p) => out.push(p.kind), 20);
	t.push(snap(['a']));
	t.push(snap());
	t.push(snap(['b']));
	await new Promise((r) => setTimeout(r, 60));
	assert.deepEqual(out, ['down']);
	t.dispose();
});

test('dev override parsing', () => {
	assert.deepEqual(parseDevOverride('?mood=warn&pulse=up'), { tone: 'warn', pulse: 'up' });
	assert.deepEqual(parseDevOverride('?mood=x'), { tone: null, pulse: null });
});
