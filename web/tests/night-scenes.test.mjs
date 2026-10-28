// The night city scenes: framing without distortion, the same sky on every load. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	BANNER_FRAME,
	STAGE_FRAME,
	WORLD_X0,
	WORLD_X1,
	blockPaths,
	fitViewBox,
	makeStars,
	moodLevel,
	pointsInTower,
	row
} from '../src/lib/components/status/scenes/nightScene.ts';

const box = (vb) => vb.split(' ').map(Number);

test('without a measured box the frame itself is shown', () => {
	assert.deepEqual(box(fitViewBox(0, 0, BANNER_FRAME)), [300, 200, 1620, 800]);
});

test('the viewBox always has the aspect of the box and stays inside the world', () => {
	for (const [w, h] of [[390, 380], [1440, 570], [1920, 570], [3840, 400], [128, 56], [1920, 1080], [800, 1280]]) {
		for (const frame of [BANNER_FRAME, STAGE_FRAME]) {
			const [x, y, vw, vh] = box(fitViewBox(w, h, frame));
			assert.ok(Math.abs(vw / vh - w / h) < 0.01, `${w}x${h}: ${vw}/${vh}`);
			assert.ok(x >= WORLD_X0 - 0.1 && x + vw <= WORLD_X1 + 0.1, `${w}x${h}: ${x}+${vw}`);
			assert.ok(y + vh <= frame.bottom + 0.1);
		}
	}
});

test('a narrow box keeps the landmark in view', () => {
	const [x, , vw] = box(fitViewBox(390, 380, BANNER_FRAME));
	assert.ok(x < 1360 && x + vw > 1360);
});

test('stars, rows and buildings are deterministic', () => {
	assert.deepEqual(makeStars(5), makeStars(5));
	assert.notDeepEqual(makeStars(5).dots, makeStars(6).dots);
	const blocks = row(0, 1000, 3, { h: [50, 100], w: [40, 80], roofs: ['flat', 'mansard', 'step', 'tank', 'slant', 'antenna'] });
	assert.ok(blocks.length > 0);
	const look = { win: [3, 4], pitch: [8, 10], lit: 0.4, seed: 2 };
	assert.deepEqual(blockPaths(blocks, look), blockPaths(blocks, look));
	assert.ok(blockPaths(blocks, look).warm.length > 0);
	assert.ok(!/NaN|undefined/.test(Object.values(blockPaths(blocks, look)).join('')));
});

test('a skipped range leaves room for the landmark', () => {
	const blocks = row(0, 2000, 4, { h: [50, 100], w: [40, 80], skip: [900, 1200] });
	assert.ok(blocks.every((b) => b.x + b.w < 900 || b.x > 1200));
});

test('sparkles fall inside the tower outline', () => {
	const pts = pointsInTower([[0, 50], [-100, 10]], 40, 1);
	assert.equal(pts.length, 40);
	for (const [x, y] of pts) assert.ok(y <= 0 && y >= -100 && Math.abs(x) <= 50);
});

test('moods map onto clear, clouds and storm', () => {
	assert.equal(moodLevel('ok'), 0);
	assert.equal(moodLevel('calm'), 0);
	assert.equal(moodLevel('warn'), 1);
	assert.equal(moodLevel('critical'), 2);
	assert.equal(moodLevel(undefined), 0);
});
