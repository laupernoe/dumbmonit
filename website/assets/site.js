/* DumbMonit product site: the little behaviour every page shares.
   No dependency. Everything here is an enhancement: without this file the
   pages read the same, nothing is hidden waiting for it.
   Motion follows the app's rules (web/src/lib/ui/motion.ts): transform and
   opacity only, and under prefers-reduced-motion the state changes without
   moving. */
(function () {
	'use strict';
	var d = document;
	var root = d.documentElement;
	var still = window.matchMedia('(prefers-reduced-motion: reduce)');
	var fine = window.matchMedia('(hover: hover) and (pointer: fine)');
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

	// ---- mobile menu ----
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
		window.matchMedia('(min-width: 900px)').addEventListener('change', function () { setMenu(false); });
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

	// ---- the mascot (home): moods, eyes that follow the pointer, a poke ----
	var bird = d.getElementById('mascot');
	if (bird) {
		var look = bird.querySelector('.m-look');
		var moodTimer;
		var mood = function (name, ms) {
			['is-happy', 'is-startled', 'is-dizzy', 'is-party'].forEach(function (c) { bird.classList.remove(c); });
			clearTimeout(moodTimer);
			if (!name) return;
			// Restart the keyframes even when the same mood comes twice in a row.
			requestAnimationFrame(function () {
				bird.classList.add(name);
				if (ms) moodTimer = setTimeout(function () { mood(null); }, ms);
			});
		};
		var pokes = [];
		bird.addEventListener('click', function () {
			var now = Date.now();
			pokes = pokes.filter(function (t) { return now - t < 2500; }).concat(now);
			if (pokes.length >= 4) {
				pokes = [];
				mood('is-dizzy', 2400);
			} else {
				mood('is-startled', 900);
			}
		});
		var cta = d.querySelector('.hero .cta .btn-primary');
		if (cta) {
			['pointerenter', 'focus'].forEach(function (ev) { cta.addEventListener(ev, function () { mood('is-happy'); }); });
			['pointerleave', 'blur'].forEach(function (ev) { cta.addEventListener(ev, function () { mood(null); }); });
		}
		var copyBtn = d.querySelector('.hero [data-copy]');
		if (copyBtn) copyBtn.addEventListener('click', function () { mood('is-party', 1200); });

		// The eyes follow a mouse; the bird rests when nobody can see it.
		var seen = true;
		var rest = function () { bird.classList.toggle('paused', !seen || d.hidden); };
		d.addEventListener('visibilitychange', rest);
		if (IO) new IntersectionObserver(function (es) { seen = es[0].isIntersecting; rest(); }).observe(bird);
		if (fine.matches && !still.matches) {
			var raf = 0, px = 0, py = 0;
			window.addEventListener('pointermove', function (e) {
				if (e.pointerType !== 'mouse' || !seen) return;
				px = e.clientX;
				py = e.clientY;
				if (raf) return;
				raf = requestAnimationFrame(function () {
					raf = 0;
					var r = bird.getBoundingClientRect();
					var dx = px - (r.left + r.width * 0.45), dy = py - (r.top + r.height * 0.4);
					var dist = Math.sqrt(dx * dx + dy * dy) || 1;
					var k = Math.min(dist / 260, 1) * 10 / dist;
					bird.classList.add('is-tracking');
					look.style.transform = 'translate(' + (dx * k).toFixed(1) + 'px,' + (dy * k).toFixed(1) + 'px)';
				});
			}, { passive: true });
			d.documentElement.addEventListener('pointerleave', function () {
				bird.classList.remove('is-tracking');
				look.style.transform = '';
			});
		}
	}

	if (still.matches) return;

	// ---- figures count up when they come into view (CountUp in the app) ----
	var nums = d.querySelectorAll('.num[data-n]');
	if (IO && nums.length) {
		var nio = new IntersectionObserver(function (entries) {
			entries.forEach(function (en) {
				if (!en.isIntersecting) return;
				var el = en.target, n = +el.dataset.n, t0 = 0;
				nio.unobserve(el);
				el.style.minWidth = el.getBoundingClientRect().width + 'px';
				var step = function (now) {
					if (!t0) t0 = now;
					var t = Math.min(1, (now - t0) / 1100);
					el.textContent = Math.round(n * (t >= 1 ? 1 : 1 - Math.pow(2, -10 * t)));
					if (t < 1) requestAnimationFrame(step);
				};
				el.textContent = '0';
				requestAnimationFrame(step);
			});
		}, { rootMargin: '0px 0px -10% 0px' });
		[].forEach.call(nums, function (el) { nio.observe(el); });
	}

	// ---- blocks and list items rise in, in cascades, as they scroll into view ----
	// Only what is below the fold at this moment is marked: nothing the reader
	// already sees ever blinks out, and without this script nothing is hidden.
	var SEL = [
		'.section-head', 'figure', '.feedback', '.notice', '.code', '.h-sm', '.dev > header', '.dev .summary',
		'.dev-grid h3', '.setup', '.dev-doc', '.cards > li', '.catalog > li', '.points > li', '.chips > li',
		'.os-list > li', '.facts > li', '.badges > li', '.fam-chips > li', '.rules > li', '.needs > li',
		'.steps > li', '.cta > *', 'main > section.block > .wrap > p', 'footer .wrap > *'
	].join(', ');
	if (IO) {
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
				el.style.setProperty('--rd', Math.min(k++, 9) * 55 + 'ms');
				el.classList.add('in');
				setTimeout(function () {
					el.classList.remove('reveal', 'in');
					el.style.removeProperty('--rd');
				}, 1400);
			});
		}, { rootMargin: '0px 0px -6% 0px' });
		marked.forEach(function (el) {
			el.classList.add('reveal');
			io.observe(el);
		});

		// ---- install: each step lights up as it reaches the reading line ----
		var steps = d.querySelector('.steps');
		if (steps) {
			steps.classList.add('lighting');
			var lio = new IntersectionObserver(function (entries) {
				entries.forEach(function (en) {
					if (en.isIntersecting) {
						en.target.classList.add('lit');
						lio.unobserve(en.target);
					}
				});
			}, { rootMargin: '0px 0px -35% 0px' });
			[].forEach.call(steps.children, function (li) { lio.observe(li); });
		}
	}

	// ---- cards and screenshots lean toward a mouse (never on touch) ----
	if (!fine.matches) return;
	[].forEach.call(d.querySelectorAll('.card, .dev-card, .shot:not(.hero-shot .shot)'), function (el) {
		var max = el.classList.contains('shot') ? 2.5 : 5;
		el.classList.add('tilt');
		el.addEventListener('pointermove', function (e) {
			if (e.pointerType !== 'mouse') return;
			var r = el.getBoundingClientRect();
			if (!r.width || !r.height) return;
			var px = (e.clientX - r.left) / r.width - 0.5;
			var py = (e.clientY - r.top) / r.height - 0.5;
			el.style.transform = 'perspective(900px) rotateX(' + (-py * 2 * max).toFixed(2) + 'deg) rotateY(' + (px * 2 * max).toFixed(2) + 'deg) scale(1.012)';
			el.style.setProperty('--mx', e.clientX - r.left + 'px');
			el.style.setProperty('--my', e.clientY - r.top + 'px');
		});
		el.addEventListener('pointerleave', function () {
			el.style.transform = '';
		});
	});
})();
