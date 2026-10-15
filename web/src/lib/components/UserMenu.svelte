<script lang="ts">
	/**
	 * The signed-in account, top right: a round avatar with the initials, and a
	 * small menu (name and role, "My account", sign out). Opens on click or tap,
	 * on hover with a mouse, and from the keyboard (Enter, Space, ↓); arrows
	 * move between items, Escape closes and gives the focus back.
	 */
	import { LogOut, UserRound } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { auth } from '#lib/stores/auth.svelte.js';

	let open = $state(false);
	/** Opened by the mouse passing over: closes again when it leaves. */
	let byHover = false;
	let leaveTimer: ReturnType<typeof setTimeout> | undefined;
	let root = $state<HTMLDivElement | null>(null);
	let button = $state<HTMLButtonElement | null>(null);
	let menu = $state<HTMLDivElement | null>(null);
	const menuId = 'user-menu';

	const name = $derived(auth.displayName);
	const initials = $derived.by(() => {
		const words = name.split(/[\s._@-]+/).filter(Boolean);
		const letters = words.length > 1 ? words[0][0] + words[1][0] : (words[0] ?? '?').slice(0, 2);
		return letters.toUpperCase();
	});
	const role = $derived(
		auth.isAdmin ? m.misc_role_admin() : auth.canOperate ? m.misc_role_operator() : m.misc_role_viewer()
	);

	function items(): HTMLElement[] {
		return menu ? [...menu.querySelectorAll<HTMLElement>('[role="menuitem"]')] : [];
	}

	function show(focusFirst = false) {
		clearTimeout(leaveTimer);
		open = true;
		if (focusFirst) requestAnimationFrame(() => items()[0]?.focus());
	}

	function hide(refocus = false) {
		clearTimeout(leaveTimer);
		open = false;
		byHover = false;
		if (refocus) button?.focus();
	}

	function onclick() {
		if (open && !byHover) hide();
		else {
			byHover = false;
			show();
		}
	}

	function onpointerenter(event: PointerEvent) {
		if (event.pointerType !== 'mouse') return;
		clearTimeout(leaveTimer);
		if (!open) {
			byHover = true;
			show();
		}
	}

	function onpointerleave(event: PointerEvent) {
		if (event.pointerType !== 'mouse' || !byHover) return;
		leaveTimer = setTimeout(() => hide(), 250);
	}

	function onbuttonkeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown' || event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			byHover = false;
			show(true);
		}
	}

	function onmenukeydown(event: KeyboardEvent) {
		const list = items();
		const index = list.indexOf(document.activeElement as HTMLElement);
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			const step = event.key === 'ArrowDown' ? 1 : -1;
			list[(index + step + list.length) % list.length]?.focus();
		} else if (event.key === 'Home' || event.key === 'End') {
			event.preventDefault();
			list[event.key === 'Home' ? 0 : list.length - 1]?.focus();
		} else if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			hide(true);
		} else if (event.key === 'Tab') {
			hide();
		}
	}

	// A tap or click anywhere else closes it.
	$effect(() => {
		if (!open) return;
		const onpointerdown = (event: PointerEvent) => {
			if (root && !root.contains(event.target as Node)) hide();
		};
		document.addEventListener('pointerdown', onpointerdown);
		return () => document.removeEventListener('pointerdown', onpointerdown);
	});

	$effect(() => () => clearTimeout(leaveTimer));
</script>

{#if auth.user}
	<!-- Hover only adds to the button: click, tap and keys open the menu too. -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="relative ml-1" bind:this={root} {onpointerenter} {onpointerleave}>
		<button
			bind:this={button}
			type="button"
			class="inline-flex size-10 items-center justify-center rounded-full transition-colors hover:bg-surface-2"
			aria-haspopup="menu"
			aria-expanded={open}
			aria-controls={open ? menuId : undefined}
			aria-label={m.misc_user_menu_label({ name })}
			{onclick}
			onkeydown={onbuttonkeydown}
		>
			<span
				class="inline-flex size-8 items-center justify-center rounded-full border border-line-strong bg-signal-soft text-[0.75rem] font-bold tracking-wide text-signal-ink"
				aria-hidden="true">{initials}</span
			>
		</button>
		{#if open}
			<div
				bind:this={menu}
				id={menuId}
				role="menu"
				tabindex="-1"
				aria-label={m.misc_user_menu_label({ name })}
				class="absolute top-full right-0 z-40 mt-1 w-64 rounded-[var(--radius-card)] border border-line bg-surface p-1.5 shadow-float"
				onkeydown={onmenukeydown}
			>
				<div class="border-b border-line px-3 pt-2 pb-2.5">
					<p class="truncate text-sm font-semibold text-ink">{name}</p>
					<p class="truncate text-[0.8125rem] text-ink-2">
						{auth.user.username} · {role}
					</p>
				</div>
				<a
					href="/settings/account"
					role="menuitem"
					class="mt-1 flex h-10 items-center gap-2.5 rounded-lg px-3 text-sm font-semibold text-ink-2 outline-none hover:bg-surface-2 hover:text-ink focus-visible:bg-surface-2 focus-visible:text-ink"
					onclick={() => hide()}
				>
					<UserRound class="size-4" aria-hidden="true" />
					{m.misc_user_menu_account()}
				</a>
				{#if auth.canSignOut}
					<button
						type="button"
						role="menuitem"
						class="flex h-10 w-full items-center gap-2.5 rounded-lg px-3 text-left text-sm font-semibold text-ink-2 outline-none hover:bg-surface-2 hover:text-ink focus-visible:bg-surface-2 focus-visible:text-ink"
						onclick={() => {
							hide();
							void auth.logout();
						}}
					>
						<LogOut class="size-4" aria-hidden="true" />
						{m.misc_nav_sign_out()}
					</button>
				{/if}
			</div>
		{/if}
	</div>
{/if}
