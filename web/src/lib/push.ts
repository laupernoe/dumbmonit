/**
 * Browser side of Web Push: what this browser can do, and the service worker
 * subscription. Network calls to the server stay in `$lib/api/webpush`.
 *
 * The worker (`static/sw.js`) only shows notifications; it caches nothing.
 */

/** Where this browser stands, from "cannot" to "ready". */
export type PushSupport =
	| { state: 'unsupported' }
	/** Not HTTPS (and not localhost): browsers hide the Push API. */
	| { state: 'insecure' }
	/** iPhone/iPad in Safari: push exists only for a site added to the Home Screen. */
	| { state: 'ios-install' }
	| { state: 'ready'; permission: NotificationPermission };

const WORKER_URL = '/sw.js';

function isIos(): boolean {
	const ua = navigator.userAgent;
	// iPadOS 13+ reports itself as a Mac; touch support gives it away.
	return /iPhone|iPad|iPod/.test(ua) || (ua.includes('Macintosh') && navigator.maxTouchPoints > 1);
}

/** True when running as an installed app (Home Screen / desktop PWA). */
export function isStandalone(): boolean {
	const nav = navigator as Navigator & { standalone?: boolean };
	return nav.standalone === true || window.matchMedia('(display-mode: standalone)').matches;
}

export function pushSupport(): PushSupport {
	if (typeof window === 'undefined') return { state: 'unsupported' };
	if (!window.isSecureContext) return { state: 'insecure' };
	const api = 'serviceWorker' in navigator && 'PushManager' in window && 'Notification' in window;
	if (!api) return isIos() && !isStandalone() ? { state: 'ios-install' } : { state: 'unsupported' };
	return { state: 'ready', permission: Notification.permission };
}

/** The worker registration, registering it on first use. */
async function registration(): Promise<ServiceWorkerRegistration> {
	const existing = await navigator.serviceWorker.getRegistration('/');
	if (existing) return existing;
	await navigator.serviceWorker.register(WORKER_URL, { scope: '/', updateViaCache: 'none' });
	return navigator.serviceWorker.ready;
}

/** This browser's current subscription, without asking anything. */
export async function currentSubscription(): Promise<PushSubscription | null> {
	const existing = await navigator.serviceWorker.getRegistration('/');
	if (!existing) return null;
	return existing.pushManager.getSubscription();
}

function decodeKey(base64url: string): Uint8Array<ArrayBuffer> {
	const base64 = base64url.replace(/-/g, '+').replace(/_/g, '/');
	const padded = base64 + '='.repeat((4 - (base64.length % 4)) % 4);
	const raw = atob(padded);
	const bytes = new Uint8Array(new ArrayBuffer(raw.length));
	for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
	return bytes;
}

function sameKey(a: ArrayBuffer | null, b: Uint8Array): boolean {
	if (!a || a.byteLength !== b.byteLength) return false;
	const view = new Uint8Array(a);
	return view.every((byte, i) => byte === b[i]);
}

/**
 * Asks for permission (must run from a click), then subscribes with the
 * server's key. A subscription made for another key (server restored on a new
 * machine) is replaced.
 */
export async function subscribe(publicKey: string): Promise<PushSubscription> {
	const permission = await Notification.requestPermission();
	if (permission !== 'granted') {
		throw new Error(
			permission === 'denied'
				? 'Notifications are blocked for this site. Allow them in the browser settings, then try again.'
				: 'Notifications were not allowed.'
		);
	}
	const key = decodeKey(publicKey);
	const reg = await registration();
	const existing = await reg.pushManager.getSubscription();
	if (existing) {
		if (sameKey(existing.options.applicationServerKey, key)) return existing;
		await existing.unsubscribe();
	}
	return reg.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: key });
}

/** Same digest as the server's `store::fingerprint`: first 16 hex chars of SHA-256(endpoint). */
export async function fingerprint(endpoint: string): Promise<string> {
	const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(endpoint));
	return Array.from(new Uint8Array(digest))
		.map((byte) => byte.toString(16).padStart(2, '0'))
		.join('')
		.slice(0, 16);
}
