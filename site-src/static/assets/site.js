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

	// ---- hero: the scripted demo (15 s loop, pauses on hover and off screen) ----
	var demo = d.getElementById('demo');
	if (demo) {
		var app = demo.querySelector('.demo-app');
		var cursor = demo.querySelector('.d-cursor');
		var row = demo.querySelector('.d-target');
		var ack = demo.querySelector('.d-ack');
		if (still.matches) {
			// A still frame: the alert has arrived and its detail is open.
			app.classList.add('s-down', 's-toast', 's-panel');
		} else {
			var LOOP = 15000, t = 0, last = 0, paused = false, onScreen = true;
			var aim = function (el, dx, dy) {
				var a = app.getBoundingClientRect(), r = el.getBoundingClientRect();
				cursor.style.transform = 'translate(' + Math.round(r.left - a.left + r.width * dx) + 'px,' + Math.round(r.top - a.top + r.height * dy) + 'px)';
			};
			var rest = function () {
				cursor.style.transform = 'translate(' + Math.round(app.clientWidth * 0.72) + 'px,' + Math.round(app.clientHeight - 40) + 'px)';
			};
			var set = function (c, on) { app.classList.toggle(c, on); };
			var steps = [
				[0, function () { app.className = 'demo-app'; void app.offsetWidth; set('s-run', true); rest(); }],
				[2500, function () { set('s-down', true); }],
				[3300, function () { set('s-toast', true); }],
				[4800, function () { aim(row, 0.42, 0.55); }],
				[5900, function () { set('s-click', true); set('s-panel', true); }],
				[6200, function () { set('s-click', false); }],
				[8300, function () { aim(ack, 0.5, 0.6); }],
				[9300, function () { set('s-click', true); set('s-ack', true); set('s-toast', false); }],
				[9600, function () { set('s-click', false); }],
				[10800, function () { set('s-down', false); }],
				[12300, function () { set('s-panel', false); rest(); }],
				[14200, function () { set('s-run', false); }]
			];
			var next = 0;
			var tick = function (now) {
				if (last && !paused && onScreen && !d.hidden) t += now - last;
				last = now;
				if (t >= LOOP) { t = 0; next = 0; }
				while (next < steps.length && t >= steps[next][0]) steps[next++][1]();
				requestAnimationFrame(tick);
			};
			demo.addEventListener('mouseenter', function () { paused = true; });
			demo.addEventListener('mouseleave', function () { paused = false; });
			if (IO) new IntersectionObserver(function (es) { onScreen = es[0].isIntersecting; }).observe(demo);
			requestAnimationFrame(tick);
		}
	}

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

/* Video tour: loads and plays only when visible, never autoplays with reduced motion. */
(function () {
	var v = document.getElementById('tour');
	if (!v) return;
	var calm = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
	if (calm || !('IntersectionObserver' in window)) {
		v.controls = true;
		return;
	}
	var io = new IntersectionObserver(function (entries) {
		entries.forEach(function (en) {
			if (en.isIntersecting) {
				v.preload = 'auto';
				var p = v.play();
				if (p && p.catch) p.catch(function () { v.controls = true; });
			} else {
				v.pause();
			}
		});
	}, { threshold: 0.35 });
	io.observe(v);
})();
