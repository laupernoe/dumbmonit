// The link between a mail server monitor and its agent. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { explicitAgent, guessAgent, isMailKind, isSilent, shortHost } from '../src/lib/components/devices/mdaemon/agent-link.ts';
import { foldAgentMail, resolveAgent, reportingQuery } from '../src/lib/components/devices/mdaemon/queues.ts';

const agent = (id, name, address = '') => ({ id, name, address, kind: 'agent', tags: {}, parent_id: null, via_agent: null });
const mail = (id, address, tags = {}, kind = 'mdaemon') => ({ id, name: 'Mail', address, kind, tags, parent_id: null, via_agent: null });

test('only the two mail kinds rely on the agent link', () => {
	assert.ok(isMailKind('mdaemon') && isMailKind('securitygateway'));
	assert.ok(!isMailKind('agent') && !isMailKind(''));
});

test('a host name is reduced to its first label, an IP stays whole', () => {
	assert.equal(shortHost('mail.example.com:444'), 'mail');
	assert.equal(shortHost('https://MAIL.example.com/x'), 'mail');
	assert.equal(shortHost('10.0.0.5:25'), '10.0.0.5');
});

test('the agent with the same host name is guessed, never an ambiguous one', () => {
	const agents = [agent(1, 'MAIL01'), agent(2, 'files'), { ...agent(3, 'srv'), kind: 'snmp' }];
	assert.equal(guessAgent('mail01.corp.local', agents)?.id, 1);
	assert.equal(guessAgent('nothing.example.com', agents), null);
	assert.equal(guessAgent('', agents), null);
	assert.equal(guessAgent('mail01', [...agents, agent(4, 'mail01.other')]), null);
	assert.equal(guessAgent('10.0.0.9', [agent(5, 'x', '10.0.0.9')])?.id, 5);
});

test('the agent_id setting names the agent, a stale id names none', () => {
	const agents = [agent(7, 'srv')];
	assert.equal(explicitAgent(mail(1, 'a', { agent_id: '7' }), agents)?.id, 7);
	assert.equal(explicitAgent(mail(1, 'a', { agent_id: '8' }), agents), null);
	assert.equal(explicitAgent(mail(1, 'a', { agent_id: 'x' }), agents), null);
	assert.equal(explicitAgent(mail(1, 'a'), agents), null);
});

test('the explicit agent wins, even before it reports anything', () => {
	const agents = [agent(7, 'srv'), agent(8, 'mail')];
	const found = resolveAgent(mail(1, 'mail.corp', { agent_id: '7' }), [...agents], new Set([8]));
	assert.deepEqual([found.agent?.id, found.explicit], [7, true]);
	const implicit = resolveAgent(mail(1, 'mail.corp'), agents, new Set([8]));
	assert.deepEqual([implicit.agent?.id, implicit.explicit], [8, false]);
	assert.equal(resolveAgent(mail(1, 'other.corp'), [], new Set()).agent, null);
});

test('an agent is silent after five minutes, or when it never reported', () => {
	const now = Date.parse('2026-10-21T12:00:00Z');
	assert.equal(isSilent(null, now), true);
	assert.equal(isSilent(new Date(now - 60_000), now), false);
	assert.equal(isSilent(new Date(now - 10 * 60_000), now), true);
});

test('the agent series fold into services, queue folders and disk space', () => {
	const serie = (name, labels, value) => ({ metric: { __name__: `dumbmonit_agent_mail_${name}`, ...labels }, values: [[0, String(value)]] });
	const out = foldAgentMail([
		serie('info', { product: 'mdaemon', version: '24.5.2' }, 1),
		serie('service_up', { service: 'MDaemon', display: 'MDaemon Server', start: 'auto' }, 1),
		serie('service_up', { service: 'MDRA', display: 'Remote Admin', start: 'manual' }, 0),
		serie('queue_files', { queue: 'inbound' }, 12),
		serie('queue_oldest_seconds', { queue: 'inbound' }, 9000),
		serie('data_free_bytes', {}, 100),
		serie('data_total_bytes', {}, 400)
	]);
	assert.equal(out.detected, true);
	assert.equal(out.version, '24.5.2');
	assert.deepEqual(out.services.map((s) => [s.name, s.running]), [['MDRA', false], ['MDaemon', true]]);
	assert.equal(out.files.inbound, 12);
	assert.equal(out.oldest.inbound, 9000);
	assert.deepEqual([out.freeBytes, out.totalBytes], [100, 400]);
});

test('the reporting query names the right series per product', () => {
	assert.match(reportingQuery('mdaemon'), /mdaemon_queue_messages/);
	assert.doesNotMatch(reportingQuery('securitygateway'), /mdaemon_queue_messages/);
});
