/*
 * DumbMonit service worker: push notifications, and nothing else.
 *
 * It exists so that alerts reach a phone or a computer with every tab closed
 * (Web Push). It deliberately has no `fetch` handler and keeps no cache: the
 * API carries live monitoring data and session-bound answers that must never
 * be served from a cache, and a monitoring screen that shows yesterday's build
 * after an upgrade is worse than one that says it is offline. Every request
 * goes to the network exactly as if this worker did not exist.
 *
 * Served from the site root so its scope covers the whole interface, with
 * `Cache-Control: no-cache` so an upgrade replaces it at the next visit.
 */

self.addEventListener('install', () => {
	// A new version takes over at once: it holds no state worth waiting for.
	self.skipWaiting();
});

self.addEventListener('activate', (event) => {
	event.waitUntil(
		(async () => {
			// Earlier experiments or other tools may have left caches behind on this
			// origin: drop them, nothing here should be served from a cache.
			const names = await caches.keys();
			await Promise.all(names.map((name) => caches.delete(name)));
			await self.clients.claim();
		})()
	);
});

/** Reads the JSON sent by the server (`notify/webpush/mod.rs`, `payload`). */
function readPayload(event) {
	if (!event.data) return {};
	try {
		return event.data.json();
	} catch {
		return { body: event.data.text() };
	}
}

/** Only same-origin relative paths are opened: a payload cannot send the user elsewhere. */
function safePath(url) {
	if (typeof url !== 'string' || !url.startsWith('/') || url.startsWith('//')) return '/alerts';
	return url;
}

self.addEventListener('push', (event) => {
	const data = readPayload(event);
	const title = typeof data.title === 'string' && data.title ? data.title : 'DumbMonit';
	const options = {
		body: typeof data.body === 'string' ? data.body : '',
		icon: '/icon-192.png',
		badge: '/icon-192.png',
		// One notification per alert: the resolution replaces the firing.
		tag: typeof data.tag === 'string' && data.tag ? data.tag : undefined,
		renotify: Boolean(data.tag) && !data.resolved,
		requireInteraction: data.severity === 'critical' && !data.resolved,
		timestamp: typeof data.timestamp === 'number' ? data.timestamp : Date.now(),
		data: { url: safePath(data.url) }
	};
	if (!options.tag) delete options.renotify;
	event.waitUntil(self.registration.showNotification(title, options));
});

self.addEventListener('notificationclick', (event) => {
	event.notification.close();
	const path = safePath(event.notification.data && event.notification.data.url);
	const target = new URL(path, self.location.origin).href;
	event.waitUntil(
		(async () => {
			const windows = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
			for (const client of windows) {
				if (new URL(client.url).origin !== self.location.origin) continue;
				await client.focus();
				if ('navigate' in client) {
					try {
						await client.navigate(target);
					} catch {
						/* Uncontrolled window: focusing it is the best we can do. */
					}
				}
				return;
			}
			await self.clients.openWindow(target);
		})()
	);
});

// The browser renewed the subscription on its own (keys rotated, expiry): the
// server no longer knows the new one. Re-subscribing here needs the server key
// and a session; the settings page does it at the next visit instead.
self.addEventListener('pushsubscriptionchange', () => {});
