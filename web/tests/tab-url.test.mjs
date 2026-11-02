// Old destinations forward to a tab of the page that absorbed them. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { tabUrl } from '../src/lib/tab-url.ts';

const from = (search = '', hash = '') => ({ searchParams: new URLSearchParams(search), hash });

test('a tab is named in ?tab= and the old query rides along', () => {
	assert.equal(tabUrl('/targets', 'map', from()), '/targets?tab=map');
	assert.equal(tabUrl('/targets', 'map', from('view=world')), '/targets?view=world&tab=map');
	assert.equal(tabUrl('/targets', 'logs', from('target=12&source=agent')), '/targets?target=12&source=agent&tab=logs');
});

test('the old ?tab= (the security page had one) is replaced, not duplicated', () => {
	assert.equal(tabUrl('/targets', 'security', from('tab=cves&target=7')), '/targets?tab=security&target=7');
});

test('the default tab needs no ?tab=', () => {
	assert.equal(tabUrl('/targets', 'list', from('tab=x'), {}, 'list'), '/targets');
});

test('extra parameters and the hash', () => {
	assert.equal(tabUrl('/alerts', 'automation', from(), { view: 'runbooks' }), '/alerts?tab=automation&view=runbooks');
	assert.equal(tabUrl('/alerts', 'rules', from('', '#r4')), '/alerts?tab=rules#r4');
});
