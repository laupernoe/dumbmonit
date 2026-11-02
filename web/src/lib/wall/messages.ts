/**
 * Translated texts for the wall's pure modules (`music.ts`, `spotify.ts`).
 * Those stay free of imports so `npm test` can run them as is; they hand out
 * a code and this file says it in the active language, at call time.
 */
import { m } from '#lib/paraglide/messages.js';
import type { MusicErrorCode } from './music';
import type { NowPlaying } from '../api/music';
import type { Playing, SdkFailure, SpeakerSupportCode } from './spotify';

export function musicErrorText(code: MusicErrorCode): string {
	switch (code) {
		case 'empty':
			return m.misc_music_empty();
		case 'not_link':
			return m.misc_music_not_link();
		case 'unsupported':
			return m.misc_music_unsupported();
		case 'spotify_short':
			return m.misc_music_spotify_short();
		case 'deezer_short':
			return m.misc_music_deezer_short();
		case 'youtube_page':
			return m.misc_music_youtube_page();
		case 'spotify_kinds':
			return m.misc_music_spotify_kinds();
		case 'spotify_cut':
			return m.misc_music_spotify_cut();
		case 'deezer_kinds':
			return m.misc_music_deezer_kinds();
		case 'deezer_cut':
			return m.misc_music_deezer_cut();
		case 'youtube_cut':
			return m.misc_music_youtube_cut();
		case 'youtube_empty':
			return m.misc_music_youtube_empty();
	}
}

export function speakerSupportText(code: SpeakerSupportCode): { reason: string; fix: string } {
	switch (code) {
		case 'insecure':
			return { reason: m.misc_speaker_reason_insecure(), fix: m.misc_speaker_fix_insecure() };
		case 'no_eme':
			return { reason: m.misc_speaker_reason_no_eme(), fix: m.misc_speaker_fix_no_drm() };
		case 'no_drm':
			return { reason: m.misc_speaker_reason_no_drm(), fix: m.misc_speaker_fix_no_drm() };
	}
}

/** The failure with its texts in the active language (a failure without a code keeps its own). */
export function sdkFailureText(failure: SdkFailure): SdkFailure {
	const detail = failure.detail ? ` (${failure.detail})` : '';
	switch (failure.code) {
		case 'initialization_error':
			return { ...failure, problem: m.misc_speaker_problem_init({ detail }), fix: m.misc_speaker_fix_no_drm() };
		case 'account_error':
			return { ...failure, problem: m.misc_speaker_problem_premium(), fix: m.misc_speaker_fix_premium() };
		case 'authentication_error':
			return { ...failure, problem: m.misc_speaker_problem_auth({ detail }), fix: m.misc_speaker_fix_auth() };
		default:
			return failure;
	}
}

/** "this display", "a phone"… for "Playing on …". */
export function deviceLabelText(now: NowPlaying, source: Playing['source'], speakerName: string): string {
	if (source === 'speaker' || (now.device_name && now.device_name === speakerName)) return m.misc_device_this();
	if (now.device_name) return now.device_name;
	switch (now.device_type) {
		case 'Smartphone':
			return m.misc_device_phone();
		case 'Tablet':
			return m.misc_device_tablet();
		case 'Computer':
			return m.misc_device_computer();
		case 'Speaker':
			return m.misc_device_speaker();
		case 'TV':
			return m.misc_device_tv();
		case 'CastVideo':
			return m.misc_device_chromecast();
		case 'CastAudio':
			return m.misc_device_cast_speaker();
		case 'AVR':
			return m.misc_device_amplifier();
		case 'GameConsole':
			return m.misc_device_console();
		default:
			return m.misc_device_other();
	}
}
