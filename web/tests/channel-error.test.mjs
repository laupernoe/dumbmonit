// Channel delivery errors: the sentence, the HTTP status and the kind of failure
// read from the server's text. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readChannelError } from '../src/lib/components/notifications/channelError.ts';

test('a service JSON body is reduced to its message, status and kind', () => {
	const raw = 'Sending to "ops" failed: the service answered 401: {"message":"Invalid token","code":0}';
	const error = readChannelError(raw);
	assert.equal(error.text, 'Sending to "ops" failed: the service answered 401: Invalid token');
	assert.equal(error.status, 401);
	assert.equal(error.kind, 'auth');
	assert.equal(error.raw, raw);
});

test('the HTTP status of the request wins over the text', () => {
	assert.equal(readChannelError('Sending failed', 429).kind, 'rate_limit');
	assert.equal(readChannelError('Sending failed', 429).status, 429);
});

test('a status 0 request (no answer) is a transport failure', () => {
	const error = readChannelError('Failed to fetch', 0);
	assert.equal(error.kind, 'transport');
	assert.equal(error.status, null);
});

test('statuses map to their kind', () => {
	const kind = (text) => readChannelError(text).kind;
	assert.equal(kind('the service answered 404: nope'), 'not_found');
	assert.equal(kind('the service answered 403'), 'auth');
	assert.equal(kind('the service answered 503'), 'server');
	assert.equal(kind('the service answered 400'), 'rejected');
});

test('local failures are named without a status', () => {
	assert.equal(readChannelError('channel configuration incomplete: missing url').kind, 'config');
	assert.equal(readChannelError('unknown channel kind: foo').kind, 'config');
	assert.equal(readChannelError('service unreachable: timed out').kind, 'transport');
	assert.equal(readChannelError('something odd').kind, 'unknown');
});

test('a message nested in an error object is found', () => {
	const error = readChannelError('the service answered 400: {"error":{"message":"chat not found"}}');
	assert.equal(error.text, 'the service answered 400: chat not found');
});

test('plain text without JSON is kept as it is', () => {
	const error = readChannelError('  service unreachable: connection refused  ');
	assert.equal(error.text, 'service unreachable: connection refused');
	assert.equal(error.status, null);
});

test('broken JSON falls back to the whole text', () => {
	const raw = 'the service answered 500: {not json';
	const error = readChannelError(raw);
	assert.equal(error.text, raw);
	assert.equal(error.status, 500);
});
