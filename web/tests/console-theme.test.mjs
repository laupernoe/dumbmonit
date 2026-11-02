// The console's theme choice: dark by default, light or the system's on request, the wall dark. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseConsoleTheme, resolveConsoleTheme } from '../src/lib/stores/theme-choice.ts';

test('anything but light or auto reads as dark', () => {
	assert.equal(parseConsoleTheme(null), 'dark');
	assert.equal(parseConsoleTheme('dark'), 'dark');
	assert.equal(parseConsoleTheme('obsidian'), 'dark');
	assert.equal(parseConsoleTheme('light'), 'light');
	assert.equal(parseConsoleTheme('auto'), 'auto');
});

test('system follows the device; a fixed choice ignores it', () => {
	assert.equal(resolveConsoleTheme('auto', true, '/'), 'dark');
	assert.equal(resolveConsoleTheme('auto', false, '/'), 'light');
	assert.equal(resolveConsoleTheme('light', true, '/alerts'), 'light');
	assert.equal(resolveConsoleTheme('dark', false, '/alerts'), 'dark');
});

test('the wall stays dark, and only the wall', () => {
	assert.equal(resolveConsoleTheme('light', false, '/wall'), 'dark');
	assert.equal(resolveConsoleTheme('auto', false, '/wall/anything'), 'dark');
	assert.equal(resolveConsoleTheme('light', false, '/wallet'), 'light');
});
