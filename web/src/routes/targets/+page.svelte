<script lang="ts">
	/**
	 * Devices — the rack.
	 *
	 * Every device is a 1U faceplate, children stacked under their parent.
	 * Targets and probe states come from the shared store (polled app-wide by
	 * the root layout, see `alertsStore`); this page only adds the sparklines,
	 * from one batched query refreshed every 30 s.
	 */
	import {
		listCollectors,
		reorderTargets,
		updateTarget,
		type Target,
		type TargetId,
		type TargetPayload
	} from '$lib/api';
	import { displayState } from '$lib/format';
	import { loadSparklines } from '$lib/metrics';
	import type { Serie } from '$lib/components/Chart.svelte';
	import { alertsStore } from '$lib/stores/alerts.svelte';
	import { auth } from '$lib/stores/auth.svelte';
	import { Button, ClickSpark, EmptyState, ErrorNotice, Menu, PageHeader, Skeleton } from '$lib/ui';
	import RackList from '$lib/components/devices/RackList.svelte';
	import type { ReorderControls } from '$lib/components/devices/RackList.svelte';
	import FolderHeader from '$lib/components/devices/FolderHeader.svelte';
	import Segmented from '$lib/components/devices/Segmented.svelte';
	import { nearestRow, needsAttention } from '$lib/components/devices/rack';
	import {
		buildFolders,
		compareFolderSections,
		folderKeyOf,
		moveWithinScope,
		reorderByDrop,
		reorderScope,
		rootOf,
		worstState
	} from '$lib/components/devices/folders';
	import { FolderPlus, Plus, Search } from 'lucide-svelte';

	type Segment = 'all' | 'attention' | 'reporting' | 'disabled';

	const targets = $derived(alertsStore.targets);
	const probes = $derived(alertsStore.probes);
	let sparklines = $state<Map<TargetId, Serie[]>>(new Map());
	let kindLabels = $state<Map<string, string>>(new Map());
	const firstLoad = $derived(alertsStore.loading && targets.length === 0);
	const pageError = $derived(!alertsStore.available ? alertsStore.lastError : null);

	let search = $state('');
	let segment = $state<Segment>('all');
	let kind = $state('');

	const stateOf = (target: Target) => displayState(target, probes.get(target.id));

	// --- Optimistic overrides for a reorder or a folder move/rename/delete:
	// applied on top of the store's `targets` until a refresh confirms them,
	// or rolled back on failure. `position` is only ever compared within one
	// reorder scope, so a sibling-local index is all an override needs. ---
	let positionOverride = $state<Map<TargetId, number>>(new Map());
	let groupOverride = $state<Map<TargetId, string>>(new Map());
	const effectiveTargets = $derived.by(() => {
		if (positionOverride.size === 0 && groupOverride.size === 0) return targets;
		return targets.map((t) => {
			const pos = positionOverride.get(t.id);
			const grp = groupOverride.get(t.id);
			if (pos === undefined && grp === undefined) return t;
			return { ...t, position: pos ?? t.position, group_name: grp ?? t.group_name };
		});
	});

	/** Kinds offered by the filter: the server's list, plus any kind a device already uses. */
	const kinds = $derived.by(() => {
		const seen = new Map<string, string>(kindLabels);
		for (const target of targets) if (!seen.has(target.kind)) seen.set(target.kind, target.kind);
		return [...seen].sort((a, b) => a[1].localeCompare(b[1], 'en'));
	});

	/** Devices matching the search box and the kind select, before the segment. */
	const narrowed = $derived.by(() => {
		const term = search.trim().toLowerCase();
		return targets.filter((target) => {
			if (kind && target.kind !== kind) return false;
			if (!term) return true;
			const label = kindLabels.get(target.kind) ?? '';
			return (
				target.name.toLowerCase().includes(term) ||
				target.address.toLowerCase().includes(term) ||
				target.kind.toLowerCase().includes(term) ||
				label.toLowerCase().includes(term)
			);
		});
	});

	const counts = $derived.by(() => {
		const c = { all: narrowed.length, attention: 0, reporting: 0, disabled: 0 };
		for (const target of narrowed) {
			const state = stateOf(target);
			if (needsAttention(state)) c.attention += 1;
			else if (state === 'online') c.reporting += 1;
			else if (state === 'disabled') c.disabled += 1;
		}
		return c;
	});

	const segments = $derived([
		{ id: 'all' as const, label: 'All', count: counts.all },
		{ id: 'attention' as const, label: 'Needs attention', count: counts.attention },
		{ id: 'reporting' as const, label: 'Reporting', count: counts.reporting },
		{ id: 'disabled' as const, label: 'Disabled', count: counts.disabled }
	]);

	const visible = $derived.by(() => {
		const ids = new Set<TargetId>();
		for (const target of narrowed) {
			const state = stateOf(target);
			const keep =
				segment === 'all' ||
				(segment === 'attention' && needsAttention(state)) ||
				(segment === 'reporting' && state === 'online') ||
				(segment === 'disabled' && state === 'disabled');
			if (keep) ids.add(target.id);
		}
		return ids;
	});

	// A folder just created in-app, with no device in it yet: there is no
	// server-side folder entity (a folder is only `Target.group_name`), so an
	// empty one exists only for this session — it is slotted in alongside the
	// real sections, in the same order, until a device is dropped into it.
	let pendingEmptyFolders = $state<Set<string>>(new Set());
	const filtered = $derived(search.trim() !== '' || segment !== 'all' || kind !== '');
	const folderSections = $derived.by(() => {
		const base = buildFolders(effectiveTargets, stateOf, visible);
		if (filtered || pendingEmptyFolders.size === 0) return base;
		const extra = [...pendingEmptyFolders]
			.filter((key) => !base.some((s) => s.key === key))
			.map((key) => ({ key, label: key, rows: [] }));
		if (extra.length === 0) return base;
		return [...base, ...extra].sort(compareFolderSections);
	});
	const totalRows = $derived(folderSections.reduce((n, s) => n + s.rows.length, 0));
	/** A single, group-less section looks exactly like the old flat list — no header shown. */
	const showFolderHeaders = $derived(folderSections.length > 1 || (folderSections[0]?.key ?? '') !== '');

	function clearFilters() {
		search = '';
		segment = 'all';
		kind = '';
	}

	// --- Folders & manual order (admin only, and only while unfiltered: the
	// reorder scope is computed from the full fleet, not the narrowed view) ---

	let collapsedFolders = $state<Set<string>>(new Set());
	function toggleFolder(key: string) {
		const next = new Set(collapsedFolders);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		collapsedFolders = next;
	}

	// "Create folder" in the toolbar: an in-app inline name field, never
	// `window.prompt`.
	let newFolderDraft = $state('');
	let newFolderInput = $state<HTMLInputElement | null>(null);

	/** Builds the payload a save needs to echo every field the server does not keep by itself. */
	function targetToPayload(target: Target, patch: Partial<TargetPayload> = {}): TargetPayload {
		return {
			name: target.name,
			address: target.address,
			kind: target.kind,
			parent_id: target.parent_id,
			via_agent: target.via_agent,
			interval_secs: target.interval_secs,
			enabled: target.enabled,
			tags: target.tags,
			group_name: target.group_name,
			...patch
		};
	}

	// A failed move, folder edit or reorder is shown above the list, in-app
	// (never a native alert); the list itself comes from the shared store and
	// stays as it was, since the optimistic overrides above are rolled back.
	let actionError = $state<unknown>(null);
	let moving = $state(false);

	async function moveToFolder(target: Target, groupName: string) {
		const current = target.group_name || '';
		if (current === groupName || moving) return;
		moving = true;
		const prev = groupOverride.get(target.id);
		const next = new Map(groupOverride);
		next.set(target.id, groupName);
		groupOverride = next;
		try {
			await updateTarget(target.id, targetToPayload(target, { group_name: groupName }));
			actionError = null;
			await alertsStore.refresh();
			const cleared = new Map(groupOverride);
			cleared.delete(target.id);
			groupOverride = cleared;
		} catch (cause) {
			const rolled = new Map(groupOverride);
			if (prev === undefined) rolled.delete(target.id);
			else rolled.set(target.id, prev);
			groupOverride = rolled;
			actionError = cause;
		} finally {
			moving = false;
		}
	}

	/** Every root device currently in folder `key` — the only ones whose own `group_name` is the folder's. */
	function rootsInFolder(key: string): Target[] {
		const roots = new Map<TargetId, Target>();
		for (const root of rootOf(effectiveTargets).values()) roots.set(root.id, root);
		return [...roots.values()].filter((r) => (r.group_name || '') === key);
	}

	async function bulkSetGroup(rows: Target[], groupName: string) {
		if (rows.length === 0 || moving) return;
		moving = true;
		const prevValues = new Map(rows.map((t) => [t.id, groupOverride.get(t.id)] as const));
		const next = new Map(groupOverride);
		for (const t of rows) next.set(t.id, groupName);
		groupOverride = next;
		try {
			await Promise.all(rows.map((t) => updateTarget(t.id, targetToPayload(t, { group_name: groupName }))));
			actionError = null;
			await alertsStore.refresh();
			const cleared = new Map(groupOverride);
			for (const t of rows) cleared.delete(t.id);
			groupOverride = cleared;
		} catch (cause) {
			const rolled = new Map(groupOverride);
			for (const [id, v] of prevValues) {
				if (v === undefined) rolled.delete(id);
				else rolled.set(id, v);
			}
			groupOverride = rolled;
			actionError = cause;
		} finally {
			moving = false;
		}
	}

	function renameFolder(oldKey: string, newKey: string) {
		const trimmed = newKey.trim();
		if (!trimmed || trimmed === oldKey) return;
		if (pendingEmptyFolders.has(oldKey)) {
			const next = new Set(pendingEmptyFolders);
			next.delete(oldKey);
			next.add(trimmed);
			pendingEmptyFolders = next;
		}
		const rows = rootsInFolder(oldKey);
		if (rows.length > 0) void bulkSetGroup(rows, trimmed);
	}

	function deleteFolder(key: string) {
		if (pendingEmptyFolders.has(key)) {
			const next = new Set(pendingEmptyFolders);
			next.delete(key);
			pendingEmptyFolders = next;
		}
		const rows = rootsInFolder(key);
		if (rows.length > 0) void bulkSetGroup(rows, '');
	}

	/** "Create folder" in the toolbar: an empty folder with nowhere to drop a
	 *  device yet, until the user drags one in (there is no server-side
	 *  folder entity to create — see `pendingEmptyFolders` above). */
	function createFolder(name: string) {
		const trimmed = name.trim();
		if (!trimmed) return;
		const next = new Set(pendingEmptyFolders);
		next.add(trimmed);
		pendingEmptyFolders = next;
		const nextCollapsed = new Set(collapsedFolders);
		nextCollapsed.delete(trimmed);
		collapsedFolders = nextCollapsed;
	}

	async function persistOrder(order: TargetId[]) {
		const prev = new Map(positionOverride);
		const next = new Map(positionOverride);
		order.forEach((id, i) => next.set(id, i));
		positionOverride = next;
		try {
			await reorderTargets(order);
			actionError = null;
			await alertsStore.refresh();
			const cleared = new Map(positionOverride);
			order.forEach((id) => cleared.delete(id));
			positionOverride = cleared;
		} catch (cause) {
			positionOverride = prev;
			actionError = cause;
		}
	}

	// --- Drag-and-drop, pointer-based (mouse and touch — HTML5 drag-and-drop
	// does not work on touch). A drag handle's pointerdown starts it straight
	// away for a mouse or a pen; a touch needs a short press first, so a plain
	// tap still just taps. Row and folder-header elements are reported by the
	// components themselves and hit-tested on every pointer move; rows never
	// reflow mid-drag, so their rects stay valid for the whole gesture. ---

	type DropSlot = { kind: 'row'; id: TargetId; before: boolean } | { kind: 'folder'; key: string };

	let dragging = $state<TargetId | null>(null);
	let dragLabel = $state('');
	let pointerPos = $state<{ x: number; y: number } | null>(null);
	let dropSlot = $state<DropSlot | null>(null);
	const rowDropSlot = $derived(dropSlot?.kind === 'row' ? dropSlot : null);

	/** Devices needing attention float to the top regardless of a manual drop: shown as a hint on the ghost. */
	const dropBlockedHint = $derived.by(() => {
		const slot = dropSlot;
		if (dragging === null || !slot || slot.kind !== 'row' || slot.id === dragging) return false;
		const dragged = effectiveTargets.find((t) => t.id === dragging);
		if (!dragged) return false;
		if (!reorderScope(effectiveTargets, dragged).some((t) => t.id === slot.id)) return false;
		return reorderByDrop(effectiveTargets, stateOf, dragging, slot.id) === null;
	});

	const rowEls = new Map<TargetId, HTMLElement>();
	const headerEls = new Map<string, HTMLElement>();
	function registerRow(id: TargetId, el: HTMLElement | null) {
		if (el) rowEls.set(id, el);
		else rowEls.delete(id);
	}
	function registerHeader(key: string, el: HTMLElement | null) {
		if (el) headerEls.set(key, el);
		else headerEls.delete(key);
	}

	function computeDropSlot(x: number, y: number): DropSlot | null {
		for (const [key, el] of headerEls) {
			const r = el.getBoundingClientRect();
			if (y >= r.top && y <= r.bottom) return { kind: 'folder', key };
		}
		const rects = [...rowEls.entries()].map(([id, el]) => {
			const r = el.getBoundingClientRect();
			return { id, top: r.top, height: r.height };
		});
		const slot = nearestRow(rects, y);
		return slot ? { kind: 'row', ...slot } : null;
	}

	let longPressTimer: ReturnType<typeof setTimeout> | null = null;
	// Set the moment a pointer drag actually starts (threshold crossed, or the
	// touch long-press fired), so the click that follows the pointerup — if
	// the browser still fires one over the same row — can be swallowed
	// instead of opening the device.
	let justDragged = false;

	/**
	 * The row itself is the drag surface, pressed anywhere on it (no handle).
	 * A mouse/pen only arms a drag once the pointer has moved past a small
	 * threshold, so a plain click still opens the device; a touch needs a
	 * short hold first (so a tap, or a scroll starting on the row, still
	 * works) and is disarmed by any early movement.
	 */
	function onRowPointerDown(id: TargetId, event: PointerEvent) {
		if (!auth.isAdmin || filtered) return;
		if (event.button !== 0) return;
		const startX = event.clientX;
		const startY = event.clientY;
		const touch = event.pointerType === 'touch';
		const threshold = touch ? 10 : 6;
		let settled = false;

		const onmove = (e: PointerEvent) => {
			if (settled) return;
			if (Math.hypot(e.clientX - startX, e.clientY - startY) <= threshold) return;
			if (touch) {
				// Moved too soon: this is a scroll, not a hold. Disarm.
				settle();
			} else {
				settle();
				beginDrag(id, e);
			}
		};
		const onup = () => settle();
		function settle() {
			settled = true;
			if (longPressTimer) clearTimeout(longPressTimer);
			longPressTimer = null;
			window.removeEventListener('pointermove', onmove);
			window.removeEventListener('pointerup', onup);
		}
		window.addEventListener('pointermove', onmove);
		window.addEventListener('pointerup', onup, { once: true });

		if (touch) {
			longPressTimer = setTimeout(() => {
				if (settled) return;
				settle();
				beginDrag(id, event);
			}, 350);
		}
	}

	function beginDrag(id: TargetId, event: PointerEvent) {
		event.preventDefault();
		const target = targets.find((t) => t.id === id);
		if (!target) return;
		justDragged = true;
		dragging = id;
		dragLabel = target.name;
		pointerPos = { x: event.clientX, y: event.clientY };
		dropSlot = computeDropSlot(event.clientX, event.clientY);
	}

	/** Swallows the click a pointer drag leaves in its wake (see `justDragged`). */
	function onRowClick(_id: TargetId, event: MouseEvent) {
		if (!justDragged) return;
		justDragged = false;
		event.preventDefault();
		event.stopPropagation();
	}

	function onPointerMove(e: PointerEvent) {
		pointerPos = { x: e.clientX, y: e.clientY };
		dropSlot = computeDropSlot(e.clientX, e.clientY);
	}

	function cancelDrag() {
		dragging = null;
		pointerPos = null;
		dropSlot = null;
		// Safety net: a click should follow the pointerup within the same
		// task, if the browser fires one at all — drop the flag afterwards so
		// it never lingers onto some later, unrelated click.
		setTimeout(() => (justDragged = false), 0);
	}

	function finishDrag() {
		const draggedId = dragging;
		const slot = dropSlot;
		cancelDrag();
		if (draggedId !== null && slot) void applyDrop(draggedId, slot);
	}

	async function applyDrop(draggedId: TargetId, slot: DropSlot) {
		const dragged = effectiveTargets.find((t) => t.id === draggedId);
		if (!dragged) return;
		if (slot.kind === 'row') {
			if (slot.id === draggedId) return;
			const order = reorderByDrop(effectiveTargets, stateOf, draggedId, slot.id);
			if (order) {
				void persistOrder(order);
				return;
			}
			// Not reorderable in place (a different folder, parent or state tier):
			// fall back to a folder-level move, root devices only — a child's
			// folder is its parent's, dragging it onto another one is a no-op.
			if (dragged.parent_id !== null) return;
			const over = effectiveTargets.find((t) => t.id === slot.id);
			if (!over) return;
			const key = folderKeyOf(effectiveTargets, over);
			if (key !== (dragged.group_name || '')) void moveToFolder(dragged, key);
			return;
		}
		if (dragged.parent_id !== null) return;
		if ((dragged.group_name || '') !== slot.key) void moveToFolder(dragged, slot.key);
	}

	let autoScrollRaf: number | null = null;
	function startAutoScroll() {
		const EDGE = 72;
		const MAX_SPEED = 16;
		const tick = () => {
			if (dragging === null) {
				autoScrollRaf = null;
				return;
			}
			if (pointerPos) {
				const { y } = pointerPos;
				const h = window.innerHeight;
				if (y < EDGE) window.scrollBy(0, -MAX_SPEED * (1 - y / EDGE));
				else if (y > h - EDGE) window.scrollBy(0, MAX_SPEED * (1 - (h - y) / EDGE));
			}
			autoScrollRaf = requestAnimationFrame(tick);
		};
		autoScrollRaf = requestAnimationFrame(tick);
	}
	function stopAutoScroll() {
		if (autoScrollRaf !== null) cancelAnimationFrame(autoScrollRaf);
		autoScrollRaf = null;
	}

	$effect(() => {
		if (dragging === null) return;
		const onpointerup = () => finishDrag();
		const onpointercancel = () => cancelDrag();
		const onkeydown = (e: KeyboardEvent) => {
			if (e.key === 'Escape') cancelDrag();
		};
		window.addEventListener('pointermove', onPointerMove);
		window.addEventListener('pointerup', onpointerup);
		window.addEventListener('pointercancel', onpointercancel);
		window.addEventListener('keydown', onkeydown);
		startAutoScroll();
		return () => {
			window.removeEventListener('pointermove', onPointerMove);
			window.removeEventListener('pointerup', onpointerup);
			window.removeEventListener('pointercancel', onpointercancel);
			window.removeEventListener('keydown', onkeydown);
			stopAutoScroll();
		};
	});

	// --- Keyboard reorder: Space picks a row up, arrows move it within its
	// scope (persisted immediately, same as a drag), Space/Enter drops it,
	// Escape restores the scope's original order. No separate handle: the
	// row itself is the one focus stop, with an aria-live announcement. ---

	let pickedUp = $state<TargetId | null>(null);
	let pickedSnapshot = $state<TargetId[] | null>(null);
	let announce = $state('');

	function onRowKeyDown(id: TargetId, event: KeyboardEvent) {
		if (!auth.isAdmin || filtered) return;
		const target = targets.find((t) => t.id === id);
		if (!target) return;
		if (event.key === ' ' || event.key === 'Enter') {
			event.preventDefault();
			if (pickedUp === id) {
				pickedUp = null;
				pickedSnapshot = null;
				announce = `Dropped ${target.name}.`;
			} else {
				pickedUp = id;
				pickedSnapshot = reorderScope(effectiveTargets, target).map((t) => t.id);
				announce = `Picked up ${target.name}. Use the up and down arrow keys to move it, space to drop, escape to cancel.`;
			}
			return;
		}
		if (pickedUp !== id) return;
		if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
			event.preventDefault();
			const delta = event.key === 'ArrowUp' ? -1 : 1;
			const order = moveWithinScope(effectiveTargets, stateOf, id, delta);
			if (order) {
				void persistOrder(order);
				announce = `${target.name} moved to position ${order.indexOf(id) + 1} of ${order.length}.`;
			} else {
				announce = `${target.name} cannot move further that way: devices needing attention stay above the rest.`;
			}
			return;
		}
		if (event.key === 'Escape') {
			event.preventDefault();
			pickedUp = null;
			if (pickedSnapshot) void persistOrder(pickedSnapshot);
			pickedSnapshot = null;
			announce = `Cancelled. ${target.name} is back where it was.`;
		}
	}

	function reorderControls(): ReorderControls {
		return {
			dragging,
			pickedUp,
			dropSlot: rowDropSlot,
			onRowPointerDown,
			onRowKeyDown,
			onRowClick,
			registerRow
		};
	}

	async function load(signal?: AbortSignal) {
		// Decoration on top of the list the shared store already holds: a
		// failure here must not take the rack down with it.
		const next = await loadSparklines(24 * 3600, signal).catch(() => null);
		if (next) sparklines = next;
	}

	async function loadKinds(signal?: AbortSignal) {
		try {
			const collectors = await listCollectors(signal);
			kindLabels = new Map(collectors.map((c) => [c.kind, c.label]));
		} catch {
			// The raw kind is a fine label until the server describes it.
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void loadKinds(controller.signal);
		void load(controller.signal);
		const timer = setInterval(() => void load(controller.signal), 30_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});
</script>

<svelte:head><title>Devices — DumbMonit</title></svelte:head>

<PageHeader title="Devices" description="Everything DumbMonit watches, stacked like a rack.">
	{#snippet actions()}
		{#if auth.isAdmin && targets.length > 0}
			<Menu label="Create a folder" align="right">
				{#snippet trigger({ toggle, open })}
					<Button
						variant="secondary"
						aria-haspopup="menu"
						aria-expanded={open}
						onclick={() => {
							toggle();
							newFolderDraft = '';
							queueMicrotask(() => newFolderInput?.focus());
						}}
					>
						<FolderPlus class="size-4" aria-hidden="true" />
						Create folder
					</Button>
				{/snippet}
				{#snippet children({ close })}
					<form
						class="flex items-center gap-1 p-0.5"
						onsubmit={(e) => {
							e.preventDefault();
							createFolder(newFolderDraft);
							newFolderDraft = '';
							close();
						}}
					>
						<input
							bind:this={newFolderInput}
							bind:value={newFolderDraft}
							class="input !h-8 min-w-0 flex-1 text-[0.8125rem]"
							placeholder="Folder name"
							maxlength="80"
							aria-label="New folder name"
							onkeydown={(e) => {
								if (e.key === 'Escape') {
									e.preventDefault();
									close();
								}
							}}
						/>
						<button
							type="submit"
							class="shrink-0 rounded-lg bg-signal px-2 py-1.5 text-[0.75rem] font-semibold text-on-signal disabled:opacity-50"
							disabled={!newFolderDraft.trim()}
						>
							Create
						</button>
					</form>
				{/snippet}
			</Menu>
		{/if}
		<!-- The empty state carries the primary itself: one primary per view. Viewers cannot add. -->
		{#if auth.isAdmin && (firstLoad || pageError || targets.length > 0)}
			<ClickSpark>
				<Button variant="primary" href="/targets/new">
					<Plus class="size-4" aria-hidden="true" />
					Add a device
				</Button>
			</ClickSpark>
		{/if}
	{/snippet}
</PageHeader>

{#if !firstLoad && !pageError && targets.length > 0}
	<div class="mb-4 flex flex-col gap-3 lg:flex-row lg:items-center">
		<label class="relative min-w-0 flex-1 lg:max-w-sm">
			<span class="sr-only">Search devices</span>
			<Search class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-ink-3" aria-hidden="true" />
			<input
				type="search"
				class="input !pl-9"
				placeholder="Search by name, address or kind"
				bind:value={search}
				autocomplete="off"
			/>
		</label>
		<div class="flex min-w-0 flex-wrap items-center gap-3">
			<Segmented options={segments} value={segment} onchange={(v) => (segment = v)} label="Filter by state" />
			<label class="min-w-0">
				<span class="sr-only">Filter by kind</span>
				<select class="input !min-h-9 !w-auto !py-1.5 text-sm" bind:value={kind}>
					<option value="">All kinds</option>
					{#each kinds as [value, label] (value)}
						<option {value}>{label}</option>
					{/each}
				</select>
			</label>
		</div>
	</div>
{/if}

{#if actionError}
	<div class="mb-3"><ErrorNotice error={actionError} title="Could not save the new order or folder" /></div>
{/if}

{#if pageError}
	<ErrorNotice
		error={pageError}
		title="Could not load the devices"
		onretry={() => void alertsStore.refresh()}
	/>
{:else if firstLoad}
	<div class="flex flex-col gap-2" aria-busy="true" aria-label="Loading devices">
		<Skeleton class="h-[66px] w-full rounded-[var(--radius-card)]" rows={5} />
	</div>
{:else if targets.length === 0}
	<EmptyState
		mascot="watch"
		title="No devices yet."
		description={auth.isAdmin ? 'Add your first switch, NAS, hypervisor or server. It takes under a minute.' : 'An admin can add the first switch, NAS, hypervisor or server.'}
	>
		{#snippet action()}
			{#if auth.isAdmin}
				<ClickSpark>
					<Button variant="primary" href="/targets/new">Add a device</Button>
				</ClickSpark>
				<p class="mt-4 text-sm text-ink-2">
					Or <a href="/targets/new?kind=agent" class="font-semibold text-ink underline decoration-line-strong underline-offset-2 hover:decoration-ink">install the agent on a machine</a>,
					or <a href="/targets/new?kind=agent&via=relay" class="font-semibold text-ink underline decoration-line-strong underline-offset-2 hover:decoration-ink">watch a remote site</a> through one.
				</p>
			{/if}
		{/snippet}
	</EmptyState>
{:else if totalRows === 0}
	<EmptyState title="Nothing matches." description="No device matches these filters.">
		{#snippet action()}
			<Button variant="ghost" onclick={clearFilters}>Clear filters</Button>
		{/snippet}
	</EmptyState>
{:else}
	<p class="sr-only" aria-live="polite">{totalRows} of {targets.length} devices shown</p>
	{#if filtered}
		<p class="mb-2 text-[0.8125rem] text-ink-2">
			<span class="tnum">{totalRows}</span> of <span class="tnum">{targets.length}</span> devices
			<button type="button" class="ml-1 text-ink-2 underline hover:text-ink" onclick={clearFilters}>Clear filters</button>
		</p>
	{/if}
	<div class="flex flex-col gap-4">
		{#each folderSections as section (section.key)}
			{@const collapsed = collapsedFolders.has(section.key)}
			{#if showFolderHeaders}
				<FolderHeader
					label={section.label}
					folderKey={section.key}
					count={section.rows.length}
					worst={worstState(section.rows)}
					{collapsed}
					ontoggle={() => toggleFolder(section.key)}
					canManage={auth.isAdmin && !filtered && section.key !== ''}
					busy={moving}
					onrename={(next) => renameFolder(section.key, next)}
					ondelete={() => deleteFolder(section.key)}
					dropActive={auth.isAdmin && dragging !== null && !filtered}
					dropHover={dragging !== null && dropSlot?.kind === 'folder' && dropSlot.key === section.key}
					onheaderref={(el) => registerHeader(section.key, el)}
				/>
			{/if}
			{#if !collapsed}
				<RackList
					rows={section.rows}
					{sparklines}
					{kindLabels}
					reorder={auth.isAdmin && !filtered ? reorderControls() : null}
				/>
			{/if}
		{/each}
	</div>
{/if}

{#if dragging !== null && pointerPos}
	<div
		class="pointer-events-none fixed z-50 flex -translate-y-1/2 items-center gap-2 rounded-[var(--radius-card)] border border-line-strong bg-surface px-3 py-2 text-sm font-semibold text-ink shadow-float"
		style={`left: ${pointerPos.x + 18}px; top: ${pointerPos.y}px;`}
	>
		{dragLabel}
		{#if dropBlockedHint}
			<span class="text-[0.75rem] font-normal text-ink-2">stays above — needs attention</span>
		{/if}
	</div>
{/if}
<p class="sr-only" aria-live="polite">{announce}</p>
