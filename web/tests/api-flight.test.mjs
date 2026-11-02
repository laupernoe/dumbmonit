import { test } from 'node:test';
import assert from 'node:assert/strict';
import { shareFlight } from '../src/lib/api/flight.ts';

/** Requests that answer when `release()` is called (all pending ones), and counts their starts. */
function gate() {
	const pending = [];
	const state = {
		starts: 0,
		aborted: 0,
		release: () => pending.splice(0).forEach((p) => p.resolve('answer')),
		fail: () => pending.splice(0).forEach((p) => p.reject(new Error('down')))
	};
	const start = (signal) => {
		state.starts += 1;
		signal.addEventListener('abort', () => (state.aborted += 1));
		return new Promise((resolve, reject) => {
			pending.push({ resolve, reject });
			signal.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')));
		});
	};
	return { state, start };
}

test('the same read asked twice at once starts one request', async () => {
	const { state, start } = gate();
	const first = shareFlight('GET /a', start);
	const second = shareFlight('GET /a', start);
	assert.equal(state.starts, 1);
	state.release();
	assert.deepEqual(await Promise.all([first, second]), ['answer', 'answer']);
});

test('nothing is kept once the answer is in', async () => {
	const { state, start } = gate();
	const first = shareFlight('GET /b', start);
	state.release();
	await first;
	const again = shareFlight('GET /b', start);
	assert.equal(state.starts, 2);
	state.release();
	await again;
});

test('different keys do not share', async () => {
	const { state, start } = gate();
	const one = shareFlight('GET /c?x=1', start);
	const two = shareFlight('GET /c?x=2', start);
	assert.equal(state.starts, 2);
	state.release();
	await Promise.allSettled([one, two]);
});

test('one caller giving up does not cancel the others', async () => {
	const { state, start } = gate();
	const quitter = new AbortController();
	const left = shareFlight('GET /d', start, quitter.signal);
	const stays = shareFlight('GET /d', start);
	quitter.abort();
	await assert.rejects(left, { name: 'AbortError' });
	assert.equal(state.aborted, 0);
	state.release();
	assert.equal(await stays, 'answer');
});

test('the request is cancelled when everybody has given up', async () => {
	const { state, start } = gate();
	const a = new AbortController();
	const b = new AbortController();
	const one = shareFlight('GET /e', start, a.signal);
	const two = shareFlight('GET /e', start, b.signal);
	a.abort();
	b.abort();
	await assert.rejects(one, { name: 'AbortError' });
	await assert.rejects(two, { name: 'AbortError' });
	assert.equal(state.aborted, 1);
	// A new caller starts a fresh request.
	const fresh = shareFlight('GET /e', start);
	assert.equal(state.starts, 2);
	state.release();
	await fresh;
});

test('a failure reaches every caller, and the next call tries again', async () => {
	const { state, start } = gate();
	const one = shareFlight('GET /f', start);
	const two = shareFlight('GET /f', start);
	state.fail();
	await assert.rejects(one, /down/);
	await assert.rejects(two, /down/);
	const again = shareFlight('GET /f', start);
	assert.equal(state.starts, 2);
	state.release();
	await again;
});

test('a caller already aborted never starts anything', async () => {
	const { state, start } = gate();
	const controller = new AbortController();
	controller.abort();
	await assert.rejects(shareFlight('GET /g', start, controller.signal), { name: 'AbortError' });
	assert.equal(state.starts, 0);
});
