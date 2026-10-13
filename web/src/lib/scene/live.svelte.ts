import {
	createPulseTracker,
	parseDevOverride,
	DEV_REPLAY_MS,
	type MoodSnapshot,
	type SceneTone,
	type ScenePulse
} from './mood.js';

/**
 * Reactive glue for a page: call during component init. `feed(snapshot)` from
 * an effect; read `pulse` / `tone` for the scene. The dev override
 * (`?mood=`, `?pulse=`) replays its pulse every 8 s.
 */
export function useSceneLive() {
	let pulse = $state<ScenePulse | null>(null);
	let tone = $state<SceneTone | null>(null);
	const tracker = createPulseTracker((p) => (pulse = p));

	$effect(() => {
		const dev = parseDevOverride(window.location.search);
		tone = dev.tone;
		if (!dev.pulse) return;
		const kind = dev.pulse;
		const play = () => (pulse = { kind, at: Date.now() });
		play();
		const timer = setInterval(play, DEV_REPLAY_MS);
		return () => clearInterval(timer);
	});
	$effect(() => () => tracker.dispose());

	return {
		get pulse() {
			return pulse;
		},
		/** Dev-forced tone, or null. */
		get tone() {
			return tone;
		},
		feed: (snapshot: MoodSnapshot) => tracker.push(snapshot),
		reset: () => tracker.reset()
	};
}
