/* DumbMonit product site: the little behaviour every page shares.
   No dependency. Everything here is an enhancement: without this file the
   pages read the same and nothing stays hidden. Motion is transform and
   opacity only; under prefers-reduced-motion nothing moves. */
(function () {
	'use strict';
	var d = document;
	var root = d.documentElement;
	var still = window.matchMedia('(prefers-reduced-motion: reduce)');
	var IO = 'IntersectionObserver' in window;

	// ---- copy buttons ----
	[].forEach.call(d.querySelectorAll('[data-copy]'), function (b) {
		var label = b.textContent;
		b.addEventListener('click', function () {
			var text = d.getElementById(b.dataset.copy).innerText.replace(/^\$ /gm, '');
			if (!navigator.clipboard) return;
			navigator.clipboard.writeText(text).then(function () {
				b.textContent = b.dataset.copied;
				b.classList.add('done');
				setTimeout(function () {
					b.textContent = label;
					b.classList.remove('done');
				}, 1600);
			}, function () {});
		});
	});

	// ---- language selector: remember the choice, close on outside click or Escape ----
	var lang = d.querySelector('.lang');
	if (lang) {
		[].forEach.call(lang.querySelectorAll('a'), function (a) {
			a.addEventListener('click', function () {
				try { localStorage.setItem('dm-lang', a.lang); } catch (e) {}
			});
		});
	}

	// ---- phone menu ----
	var menuBtn = d.querySelector('.menu-btn');
	function setMenu(open) {
		if (!menuBtn) return;
		menuBtn.setAttribute('aria-expanded', open ? 'true' : 'false');
		root.classList.toggle('menu-open', open);
	}
	if (menuBtn) {
		menuBtn.addEventListener('click', function () {
			setMenu(menuBtn.getAttribute('aria-expanded') !== 'true');
		});
		window.matchMedia('(min-width: 1200px)').addEventListener('change', function () { setMenu(false); });
	}
	d.addEventListener('click', function (e) {
		if (lang && lang.open && !lang.contains(e.target)) lang.open = false;
		if (root.classList.contains('menu-open') && !e.target.closest('.nav, .menu-btn')) setMenu(false);
	});
	d.addEventListener('keydown', function (e) {
		if (e.key !== 'Escape') return;
		if (lang && lang.open) {
			lang.open = false;
			lang.querySelector('summary').focus();
		}
		if (root.classList.contains('menu-open')) {
			setMenu(false);
			menuBtn.focus();
		}
	});

	// ---- hero: the word that turns, every 2.5 s ----
	var rot = d.querySelector('.hero .rot');
	if (rot && !still.matches) {
		var words = rot.children, wi = 0;
		setInterval(function () {
			if (d.hidden) return;
			var cur = words[wi];
			wi = (wi + 1) % words.length;
			cur.classList.remove('on');
			cur.classList.add('off');
			words[wi].classList.remove('off');
			words[wi].classList.add('on');
			setTimeout(function () { cur.classList.remove('off'); }, 500);
		}, 2500);
	}

	// ---- hero: the demo console ----
	// It plays a short script by itself (15 s loop, paused on hover and off screen); the first
	// click, tap or keyboard focus inside hands it over: pick a row to see its detail,
	// acknowledge the alert, close the detail (Escape), or Replay. After a while without
	// input the script starts again. Under reduced motion there is no script: a still frame
	// (alert open on nas-02) that can be used the same way.
	var demo = d.getElementById('demo');
	if (demo) (function () {
		var $ = function (s) { return demo.querySelector(s); };
		var app = $('.demo-app'), cursor = $('.d-cursor'), panel = $('.d-panel'), toast = $('.d-toast');
		var ackBtn = $('.d-ack'), quiet = $('.d-quiet'), chart = $('.d-chart'), line = $('.d-line');
		var pName = $('.d-pn'), pType = $('.d-pt'), pState = $('.d-pst');
		var rows = [].slice.call(demo.querySelectorAll('.d-list li'));
		var target = $('.d-target');
		var IDLE = 20000;
		var S, mode = 'auto', idleTimer = 0, hovered = false, drawn = '';

		// The graphs: the alerting row climbs to the timeout line; the others hum along.
		var SPIKE = 'M0 46 L16 44 L32 47 L48 43 L64 45 L80 42 L96 46 L112 41 L128 44 L144 40 L156 43 L166 30 L174 12 L184 7 L240 7';
		var BACK = 'M0 46 L16 44 L32 47 L48 43 L64 45 L80 42 L96 46 L112 41 L128 44 L144 40 L156 43 L166 30 L174 12 L184 7 L204 7 L212 40 L224 44 L240 43';
		function calm(seed, ms) {
			var y = 48 - Math.min(14, Math.log(1 + ms) * 3), out = [], r = seed * 9301 + 49297;
			for (var x = 0; x <= 240; x += 16) {
				r = (r * 9301 + 49297) % 233280;
				out.push((x ? 'L' : 'M') + x + ' ' + (y + (r / 233280 - 0.5) * 8).toFixed(1));
			}
			return out.join(' ');
		}
		function fresh() { return { down: false, acked: false, was: false, toast: false, sel: null }; }
		S = fresh();

		function stateOf(li) {
			if (li !== target) return 'up';
			return S.acked ? 'ack' : S.down ? 'down' : 'up';
		}
		function render() {
			rows.forEach(function (li) {
				var st = stateOf(li);
				li.classList.toggle('st-down', st === 'down');
				li.classList.toggle('st-ack', st === 'ack');
				li.firstElementChild.setAttribute('aria-pressed', li === S.sel ? 'true' : 'false');
			});
			app.classList.toggle('s-toast', S.toast);
			app.classList.toggle('s-panel', !!S.sel);
			toast.inert = !S.toast;
			panel.inert = !S.sel;
			if (!S.sel) { drawn = ''; return; }
			var li = S.sel, st = stateOf(li);
			pName.textContent = li.querySelector('.nm').textContent;
			pType.textContent = li.querySelector('.ty').textContent;
			panel.setAttribute('aria-label', pName.textContent);
			var mark = li.querySelector(st === 'down' ? '.mk.d-down' : st === 'ack' ? '.mk.d-acked' : '.mk.d-up');
			pState.innerHTML = '';
			pState.appendChild(mark.cloneNode(true));
			var alerting = li === target && (S.down || S.acked);
			ackBtn.hidden = !alerting;
			quiet.hidden = alerting;
			ackBtn.classList.toggle('done', S.acked);
			ackBtn.setAttribute('aria-disabled', S.acked ? 'true' : 'false');
			var path = li === target ? (alerting ? SPIKE : S.was ? BACK : calm(2, 12)) : calm(rows.indexOf(li) + 3, +li.dataset.ms);
			chart.classList.toggle('calm', li !== target || (!alerting && !S.was));
			var key = rows.indexOf(li) + ':' + path;
			if (key !== drawn) {
				// Uncover the new curve from the left, in one piece.
				drawn = key;
				line.setAttribute('d', path);
				chart.classList.remove('drawn');
				void chart.offsetWidth;
				chart.classList.add('drawn');
			}
		}
		function select(li) { S.sel = li; render(); }
		function acknowledge() {
			if (!S.down || S.acked) return;
			S.acked = true;
			S.toast = false;
			render();
		}

		// ---- the script ----
		var t = 0, last = 0, onScreen = true, next = 0, LOOP = 15000;
		function aim(el, dx, dy) {
			var a = app.getBoundingClientRect(), r = el.getBoundingClientRect();
			cursor.style.transform = 'translate(' + Math.round(r.left - a.left + r.width * dx) + 'px,' + Math.round(r.top - a.top + r.height * dy) + 'px)';
		}
		function rest() {
			cursor.style.transform = 'translate(' + Math.round(app.clientWidth * 0.72) + 'px,' + Math.round(app.clientHeight - 40) + 'px)';
		}
		function cls(c, on) { app.classList.toggle(c, on); }
		var steps = [
			[0, function () { S = fresh(); drawn = ''; cls('s-run', false); void app.offsetWidth; cls('s-run', true); render(); rest(); }],
			[2500, function () { S.down = true; render(); }],
			[3300, function () { S.toast = true; render(); }],
			[4800, function () { aim(target, 0.42, 0.55); }],
			[5900, function () { cls('s-click', true); select(target); }],
			[6200, function () { cls('s-click', false); }],
			[8300, function () { aim(ackBtn, 0.5, 0.6); }],
			[9300, function () { cls('s-click', true); acknowledge(); }],
			[9600, function () { cls('s-click', false); }],
			[10800, function () { S.down = false; S.acked = false; S.was = true; render(); }],
			[12300, function () { select(null); rest(); }],
			[14200, function () { cls('s-run', false); }]
		];
		function tick(now) {
			if (mode === 'auto') {
				if (last && !hovered && onScreen && !d.hidden) t += now - last;
				if (t >= LOOP) { t = 0; next = 0; }
				while (next < steps.length && t >= steps[next][0]) steps[next++][1]();
			}
			last = now;
			requestAnimationFrame(tick);
		}

		// ---- handing over to the visitor, and back ----
		function stillFrame() {
			// The frame shown under reduced motion, and after Replay there.
			S = fresh();
			S.down = S.toast = true;
			S.sel = target;
			drawn = '';
			render();
		}
		function takeOver() {
			if (mode === 'manual') return;
			mode = 'manual';
			app.classList.remove('s-run', 's-click');
			app.classList.add('s-manual');
			// Something to act on: if the script was between alerts, the alert is live again.
			if (!S.down && !S.acked) { S.down = true; S.toast = true; S.was = false; render(); }
		}
		function play() {
			mode = 'auto';
			app.classList.remove('s-manual');
			t = 0; next = 0;
		}
		function poke() {
			clearTimeout(idleTimer);
			if (still.matches) return;
			idleTimer = setTimeout(function wake() {
				// Never take the demo back from someone using it (focus inside, pointer over it).
				if (demo.contains(d.activeElement) || hovered) { idleTimer = setTimeout(wake, IDLE); return; }
				play();
			}, IDLE);
		}

		demo.addEventListener('pointerdown', function () { takeOver(); poke(); }, true);
		demo.addEventListener('focusin', function () { takeOver(); poke(); });
		demo.addEventListener('keydown', function (e) {
			poke();
			var i = rows.indexOf(e.target.parentElement);
			if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && i > -1) {
				e.preventDefault();
				var n = rows[(i + (e.key === 'ArrowDown' ? 1 : rows.length - 1)) % rows.length];
				n.firstElementChild.focus();
			} else if (e.key === 'Escape' && S.sel) {
				var back = S.sel;
				select(null);
				back.firstElementChild.focus();
			}
		});
		rows.forEach(function (li) {
			li.firstElementChild.addEventListener('click', function () { select(S.sel === li ? null : li); });
		});
		toast.addEventListener('click', function () { select(target); });
		ackBtn.addEventListener('click', acknowledge);
		$('.d-x').addEventListener('click', function () {
			var back = S.sel;
			select(null);
			if (back) back.firstElementChild.focus();
		});
		$('.d-replay').addEventListener('click', function () {
			clearTimeout(idleTimer);
			if (still.matches) { mode = 'manual'; app.classList.add('s-manual'); stillFrame(); return; }
			play();
		});
		demo.addEventListener('mouseenter', function () { hovered = true; });
		demo.addEventListener('mouseleave', function () { hovered = false; });

		if (still.matches) {
			mode = 'manual';
			app.classList.add('s-manual');
			stillFrame();
		} else {
			render();
			if (IO) new IntersectionObserver(function (es) { onScreen = es[0].isIntersecting; }).observe(demo);
			requestAnimationFrame(tick);
		}
	})();

	if (!IO) return;

	// ---- the alert path lights its steps only while it is on screen ----
	var plays = d.querySelectorAll('.steps-dg');
	if (plays.length && !still.matches) {
		var pio = new IntersectionObserver(function (entries) {
			entries.forEach(function (en) { en.target.classList.toggle('play', en.isIntersecting); });
		}, { rootMargin: '0px 0px -15% 0px' });
		[].forEach.call(plays, function (el) { pio.observe(el); });
	}

	if (still.matches) return;

	// ---- blocks settle in, in short cascades, as they scroll into view ----
	// Only what is below the fold right now is marked: nothing already on screen blinks.
	var SEL = [
		'.section-head', '.dg-fig', '.feedback', '.notice', '.code', '.h-sm', '.dev', '.cells > li',
		'.cards > li', '.aud > li', '.catalog > li', '.points > li', '.os-list > li', '.facts > li',
		'.wallets > li', '.steps > li', '.rm-sec', '.cta > *'
	].join(', ');
	var vh = window.innerHeight;
	var marked = [].filter.call(d.querySelectorAll(SEL), function (el) {
		if (el.closest('.hero, .page-head, .filter, details:not([open])')) return false;
		if (el.parentElement.closest(SEL)) return false;
		return el.getBoundingClientRect().top > vh;
	});
	var io = new IntersectionObserver(function (entries) {
		var k = 0;
		entries.forEach(function (en) {
			if (!en.isIntersecting) return;
			var el = en.target;
			io.unobserve(el);
			el.style.setProperty('--rd', Math.min(k++, 8) * 50 + 'ms');
			el.classList.add('in');
			setTimeout(function () {
				el.classList.remove('reveal', 'in');
				el.style.removeProperty('--rd');
			}, 1200);
		});
	}, { rootMargin: '0px 0px -6% 0px' });
	marked.forEach(function (el) {
		el.classList.add('reveal');
		io.observe(el);
	});
})();
