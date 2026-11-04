// Synology NAS read by the agent package: folding its series for the panel. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { dsmQuery, foldDsm } from '../src/lib/components/devices/agent/dsm.ts';

const serie = (name, labels, value) => ({
	metric: { __name__: `dumbmonit_${name}`, target: '7', ...labels },
	values: [[1_760_000_000, String(value)]]
});

test('a machine that is not a Synology shows nothing', () => {
	assert.equal(foldDsm([]), null);
	assert.equal(foldDsm([serie('filesystem_used_percent', { mountpoint: '/volume1' }, 40)]), null);
});

test('storage, disks, packages and the UPS are folded per item', () => {
	const view = foldDsm([
		serie('agent_dsm_info', { model: 'DS920+', dsm_version: 'DSM 7.2.1-69057 Update 5' }, 1),
		serie('agent_dsm_storage_health', {}, 2),
		serie('agent_dsm_raid_status', { array: 'md0', level: 'raid1', role: 'system' }, 0),
		serie('agent_dsm_raid_status', { array: 'md2', level: 'raid5', role: 'data' }, 2),
		serie('agent_dsm_raid_members', { array: 'md2' }, 4),
		serie('agent_dsm_raid_failed_member_info', { array: 'md2', member: 'sata4p5' }, 1),
		serie('agent_dsm_raid_sync_percent', { array: 'md2', action: 'recovery' }, 12.6),
		serie('agent_dsm_volume_read_only', { volume: '/volume1', fstype: 'btrfs' }, 0),
		serie('filesystem_used_percent', { mountpoint: '/volume1', fstype: 'btrfs' }, 91),
		serie('filesystem_used_percent', { mountpoint: '/volume1/@docker', fstype: 'btrfs' }, 91),
		serie('agent_dsm_disk_info', { disk: 'sata10', model: 'WD80EFZZ', kind: 'hdd' }, 1),
		serie('agent_dsm_disk_info', { disk: 'sata2', model: 'WD80EFZZ', kind: 'hdd' }, 1),
		serie('agent_dsm_disk_temperature_celsius', { disk: 'sata2' }, 41),
		serie('agent_dsm_package_info', { package: 'Plex', name: 'Plex Media Server', version: '1.40' }, 1),
		serie('agent_dsm_package_info', { package: 'HyperBackup', name: 'Hyper Backup' }, 1),
		serie('agent_dsm_package_info', { package: 'Zzz', name: 'Zzz' }, 1),
		serie('agent_dsm_package_enabled', { package: 'Zzz' }, 1),
		serie('agent_dsm_package_running', { package: 'Zzz' }, 0),
		serie('agent_dsm_package_down', { package: 'Zzz' }, 1),
		serie('agent_dsm_service_running', { service: 'smb' }, 1),
		serie('agent_sensor_temperature_celsius', { sensor: 'coretemp Package id 0' }, 48),
		serie('agent_sensor_temperature_celsius', { sensor: 'coretemp Core 0' }, 52),
		serie('nut_ups_info', { ups: 'ups', model: 'Back-UPS ES 700G' }, 1),
		serie('nut_ups_on_battery', { ups: 'ups' }, 1),
		serie('nut_battery_charge_percent', { ups: 'ups' }, 87)
	]);
	assert.ok(view);
	assert.equal(view.model, 'DS920+');
	assert.equal(view.storageHealth, 2);
	assert.equal(view.temperature, 52, 'the hottest sensor');
	assert.deepEqual(view.arrays.map((a) => a.name), ['md2', 'md0'], 'data arrays first');
	assert.deepEqual(view.arrays[0].failed, ['sata4p5']);
	assert.deepEqual(view.arrays[0].sync, { action: 'recovery', percent: 12.6 });
	assert.deepEqual(view.volumes.map((v) => v.mount), ['/volume1'], 'only /volumeN mounts');
	assert.equal(view.volumes[0].usedPercent, 91);
	assert.deepEqual(view.disks.map((d) => d.name), ['sata2', 'sata10'], 'natural order');
	assert.equal(view.disks[0].temperature, 41);
	assert.deepEqual(view.packages.map((p) => p.id), ['Zzz', 'HyperBackup', 'Plex'], 'down first, then key packages');
	assert.equal(view.packages[2].running, null, 'unknown state is not "stopped"');
	assert.deepEqual(view.ups[0], {
		name: 'ups',
		model: 'Back-UPS ES 700G',
		onBattery: true,
		lowBattery: false,
		charge: 87,
		runtime: null,
		load: null
	});
});

test('the query reads one machine and only its /volumeN mounts', () => {
	const query = dsmQuery(7);
	assert.match(query, /target="7"/);
	assert.match(query, /agent_dsm_\.\+/);
	assert.match(query, /mountpoint=~"\(\/volume\[0-9\]\+\)\?"/);
});
