/**
 * Pure part of the backup matrix (jobs x days): turns the stored series of a
 * backup integration (Veeam, Duplicati, Kopia, Active Backup, Plakar) into one
 * row per job and one state per day. No Svelte, no network: the component
 * loads the series, this module reads them. Tested in `tests/backup-history.test.mjs`.
 *
 * None of those integrations stores a per-day history of its own, so the
 * history is rebuilt from the series the collectors already write:
 *
 * - a "run age" series (seconds since the last run, any outcome) or, failing
 *   that, a "success age" series (seconds since the last success): each
 *   distinct `sample time - age` is one run;
 * - "failed" / "warning" / "running" flags sampled at the same instants give the
 *   outcome of each run (read on the first sample that saw the job idle after it,
 *   so a run in progress is not judged on the previous run's result);
 * - optional "duration" and "bytes" series give the figures shown on hover.
 *
 * Without a run age the failures cannot be dated: a day then counts as failed
 * when a sample saw the failed flag raised and no run succeeded that day.
 */

export type DayState = 'ok' | 'warning' | 'failed' | 'running' | 'none' | 'nodata';
export type RunOutcome = 'ok' | 'warning' | 'failed' | 'running';

export interface MatrixRun {
	/** Unix seconds. */
	at: number;
	outcome: RunOutcome;
	durationSeconds: number | null;
	bytes: number | null;
	/** Extra words from the source (an error excerpt), already short. */
	note?: string;
}

export interface MatrixDay {
	/** `YYYY-MM-DD`, in the viewer's time zone. */
	date: string;
	state: DayState;
	runs: MatrixRun[];
	/** More lines for the tooltip, already translated by the adapter. */
	notes?: string[];
}

export interface MatrixJob {
	key: string;
	title: string;
	subtitle: string;
	/** Oldest first, today last. */
	days: MatrixDay[];
	/** Unix seconds of the latest successful (or warned) run. */
	lastSuccess: number | null;
	/** The newest known result is a failure. */
	failing: boolean;
	running: boolean;
	durationSeconds: number | null;
	bytes: number | null;
}

/** `[unix seconds, value]`. */
export type Sample = [number, number];

export interface JobSeries {
	key: string;
	title: string;
	subtitle: string;
	runAge?: Sample[];
	successAge?: Sample[];
	failed?: Sample[];
	warning?: Sample[];
	running?: Sample[];
	duration?: Sample[];
	bytes?: Sample[];
}

export type Role = 'runAge' | 'successAge' | 'failed' | 'warning' | 'running' | 'duration' | 'bytes';

export interface RoleSpec {
	/** Full metric name, with its `dumbmonit_` prefix. */
	metric: string;
	/** Maps the stored value (a status code) to the role's value. */
	map?: (value: number) => number;
}

/** How one integration's series map to the matrix. */
export interface HistorySource {
	id: string;
	roles: Partial<Record<Role, RoleSpec>>;
	/** Labels whose values together identify a job. */
	keyLabels: string[];
	titleLabel: string;
	subtitleLabel?: string;
	/** Series to leave out (a Plakar kloset that cannot be opened has source `*`). */
	skip?: (labels: Record<string, string>) => boolean;
}

interface RawSeries {
	metric: Record<string, string>;
	values: [number, string][];
}

const DAY = 86_400;
/** Two samples of one run can disagree by about a probe interval; beyond this they are two runs. */
const SAME_RUN_SECONDS = 900;
/** The window a source asks for: thirty days. */
export const HISTORY_DAYS = 30;

const flag = (v: number) => (v >= 1 ? 1 : 0);

/** The sources wired to the matrix; every metric name is checked against `crates/collectors` and `crates/agent`. */
export const SOURCES: Record<string, HistorySource> = {
	veeam: {
		id: 'veeam',
		roles: {
			runAge: { metric: 'dumbmonit_veeam_job_last_run_age_seconds' },
			failed: { metric: 'dumbmonit_veeam_job_failed' },
			warning: { metric: 'dumbmonit_veeam_job_warning' },
			running: { metric: 'dumbmonit_veeam_job_running' },
			duration: { metric: 'dumbmonit_veeam_job_last_duration_seconds' }
		},
		keyLabels: ['job'],
		titleLabel: 'job',
		subtitleLabel: 'type'
	},
	duplicati: {
		id: 'duplicati',
		roles: {
			successAge: { metric: 'dumbmonit_duplicati_backup_last_success_age_seconds' },
			failed: { metric: 'dumbmonit_duplicati_backup_failed' },
			warning: { metric: 'dumbmonit_duplicati_backup_last_warnings', map: flag },
			running: { metric: 'dumbmonit_duplicati_backup_running' },
			duration: { metric: 'dumbmonit_duplicati_backup_last_duration_seconds' },
			bytes: { metric: 'dumbmonit_duplicati_backup_source_bytes' }
		},
		keyLabels: ['backup'],
		titleLabel: 'backup',
		subtitleLabel: ''
	},
	kopia: {
		id: 'kopia',
		roles: {
			runAge: { metric: 'dumbmonit_kopia_source_last_snapshot_age_seconds' },
			// Kopia keeps a snapshot with unreadable files: a warning, not a failure.
			warning: { metric: 'dumbmonit_kopia_source_last_snapshot_errors', map: flag },
			running: { metric: 'dumbmonit_kopia_source_uploading' },
			duration: { metric: 'dumbmonit_kopia_source_last_snapshot_duration_seconds' },
			bytes: { metric: 'dumbmonit_kopia_source_last_snapshot_bytes' }
		},
		keyLabels: ['source'],
		titleLabel: 'source'
	},
	// `abb_task_last_status`: 1 success, 0 failure (or partial), 2 running, -1 unknown.
	abb: {
		id: 'abb',
		roles: {
			successAge: { metric: 'dumbmonit_abb_task_last_success_seconds' },
			failed: { metric: 'dumbmonit_abb_task_last_status', map: (v) => (v === 0 ? 1 : 0) },
			running: { metric: 'dumbmonit_abb_task_last_status', map: (v) => (v === 2 ? 1 : 0) }
		},
		keyLabels: ['task_id', 'task'],
		titleLabel: 'task',
		subtitleLabel: ''
	},
	// `backup_last_status`: 1 snapshot without errors, 0 errors or unreadable kloset.
	plakar: {
		id: 'plakar',
		roles: {
			successAge: { metric: 'dumbmonit_backup_last_success_seconds' },
			failed: { metric: 'dumbmonit_backup_last_status', map: (v) => (v < 1 ? 1 : 0) }
		},
		keyLabels: ['kloset', 'source'],
		titleLabel: 'source',
		subtitleLabel: 'kloset',
		skip: (labels) => labels.source === '*'
	}
};

/** Names of the metrics a source reads, for the `__name__` selector. */
export function metricNames(source: HistorySource): string[] {
	return [...new Set(Object.values(source.roles).map((r) => r.metric))];
}

/** Groups raw range series into one `JobSeries` per job, sorted by title. */
export function groupSeries(series: RawSeries[], source: HistorySource): JobSeries[] {
	const byKey = new Map<string, JobSeries>();
	for (const s of series) {
		const name = s.metric.__name__ ?? '';
		if (source.skip?.(s.metric)) continue;
		const matching = (Object.entries(source.roles) as [Role, RoleSpec][]).filter(
			([, spec]) => spec.metric === name
		);
		if (matching.length === 0) continue;
		const key = source.keyLabels.map((l) => s.metric[l] ?? '').join('\u0000');
		const title = s.metric[source.titleLabel] ?? '';
		if (!title) continue;
		let job = byKey.get(key);
		if (!job) {
			job = {
				key,
				title,
				subtitle: source.subtitleLabel ? (s.metric[source.subtitleLabel] ?? '') : ''
			};
			byKey.set(key, job);
		}
		for (const [role, spec] of matching) {
			const samples: Sample[] = [];
			for (const [t, raw] of s.values) {
				const v = Number(raw);
				if (Number.isFinite(t) && Number.isFinite(v)) samples.push([t, spec.map ? spec.map(v) : v]);
			}
			job[role] = (job[role] ?? []).concat(samples).sort((a, b) => a[0] - b[0]);
		}
	}
	return [...byKey.values()].sort((a, b) => a.title.localeCompare(b.title, 'en'));
}

/** `YYYY-MM-DD` of a Unix instant in a zone given as minutes east of UTC. */
export function localDate(seconds: number, offsetMinutes: number): string {
	return new Date((seconds + offsetMinutes * 60) * 1000).toISOString().slice(0, 10);
}

function valueMap(samples: Sample[] | undefined): Map<number, number> {
	return new Map(samples ?? []);
}

interface Cluster {
	at: number;
	samples: number[];
}

/** Rebuilds a job's days from its series. See the module comment for the method. */
export function deriveJob(
	job: JobSeries,
	options: { days?: number; nowSeconds: number; offsetMinutes?: number }
): MatrixJob {
	const days = options.days ?? HISTORY_DAYS;
	const offset = options.offsetMinutes ?? 0;
	const now = options.nowSeconds;
	const todayStart = Math.floor((now + offset * 60) / DAY) * DAY - offset * 60;
	const firstStart = todayStart - (days - 1) * DAY;
	const dayOf = (t: number): number | null => {
		const index = Math.floor((t - firstStart) / DAY);
		return index >= 0 && index < days ? index : null;
	};

	const failed = valueMap(job.failed);
	const warning = valueMap(job.warning);
	const running = valueMap(job.running);
	const duration = valueMap(job.duration);
	const bytes = valueMap(job.bytes);

	const base = job.runAge ?? job.successAge ?? [];
	const firstSample = Math.min(
		...[job.runAge, job.successAge, job.failed, job.warning, job.running, job.duration, job.bytes]
			.flatMap((s) => (s && s.length > 0 ? [s[0][0]] : []))
	);

	// One cluster per run: samples whose `t - age` agree within a probe interval.
	const clusters: Cluster[] = [];
	for (const [t, age] of base) {
		const at = Math.round(t - age);
		const last = clusters.at(-1);
		if (last && Math.abs(at - last.at) <= SAME_RUN_SECONDS) {
			last.at = Math.min(last.at, at);
			last.samples.push(t);
		} else {
			clusters.push({ at, samples: [t] });
		}
	}

	const runs: MatrixRun[] = clusters.map((c) => {
		// The first sample that saw the job idle holds the verdict of this run: the
		// flags describe the latest result, which a later run (or, without a run age,
		// a later failure) overwrites.
		const idle = c.samples.find((t) => (running.get(t) ?? 0) < 1);
		let outcome: RunOutcome = 'running';
		if (idle !== undefined) {
			outcome = (failed.get(idle) ?? 0) >= 1 ? 'failed' : (warning.get(idle) ?? 0) >= 1 ? 'warning' : 'ok';
		}
		const readAt = idle ?? c.samples.at(-1) ?? c.at;
		return {
			at: c.at,
			outcome,
			durationSeconds: duration.get(readAt) ?? null,
			bytes: bytes.get(readAt) ?? null
		};
	});

	const cells: MatrixDay[] = Array.from({ length: days }, (_, i) => ({
		date: localDate(firstStart + i * DAY + 1, offset),
		state: 'none' as DayState,
		runs: []
	}));
	for (const run of runs) {
		const index = dayOf(run.at);
		if (index !== null) cells[index].runs.push(run);
	}
	// No run age: the failed flag is the only trace of a failure, dated by its sample.
	const failedDays = new Set<number>();
	if (!job.runAge) {
		for (const [t, v] of failed) {
			const index = dayOf(t);
			if (v >= 1 && index !== null) failedDays.add(index);
		}
	}
	const firstIndex = Number.isFinite(firstSample) ? Math.floor((firstSample - firstStart) / DAY) : days;
	cells.forEach((cell, i) => {
		const outcomes = cell.runs.map((r) => r.outcome);
		if (outcomes.includes('ok')) cell.state = 'ok';
		else if (outcomes.includes('warning')) cell.state = 'warning';
		else if (outcomes.includes('failed') || failedDays.has(i)) cell.state = 'failed';
		else if (outcomes.includes('running')) cell.state = 'running';
		else cell.state = i < firstIndex ? 'nodata' : 'none';
	});

	const succeeded = runs.filter((r) => r.outcome === 'ok' || r.outcome === 'warning');
	const lastSuccess = succeeded.length > 0 ? Math.max(...succeeded.map((r) => r.at)) : null;
	const lastOf = (samples: Sample[] | undefined) => samples?.at(-1)?.[1] ?? null;
	return {
		key: job.key,
		title: job.title,
		subtitle: job.subtitle,
		days: cells,
		lastSuccess,
		failing: (lastOf(job.failed) ?? 0) >= 1,
		running: (lastOf(job.running) ?? 0) >= 1,
		durationSeconds: lastOf(job.duration),
		bytes: lastOf(job.bytes)
	};
}
