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

	// ---- the sky (home): a click pokes the pigeon; it rests when nobody can see it ----
	var sky = d.getElementById('sky');
	if (sky) {
		var visible = true;
		var pause = function () { sky.classList.toggle('paused', !visible || d.hidden); };
		d.addEventListener('visibilitychange', pause);
		if (IO) new IntersectionObserver(function (es) { visible = es[0].isIntersecting; pause(); }).observe(sky);
		var flight = sky.querySelector('.flight');
		var timer;
		sky.addEventListener('click', function () {
			if (still.matches) return;
			// A roll the reader cannot see is wasted: bring the bird back into view first.
			var a = flight.getAnimations && flight.getAnimations()[0];
			var t = a && a.effect && a.effect.getComputedTiming();
			if (t && typeof t.duration === 'number') {
				var x = -36 + 272 * (t.progress || 0);
				if (x < 20 || x > 180) a.currentTime = t.duration * 0.55;
			}
			sky.classList.remove('poke');
			requestAnimationFrame(function () {
				sky.classList.add('poke');
				clearTimeout(timer);
				timer = setTimeout(function () { sky.classList.remove('poke'); }, 700);
			});
		});
	}

	if (still.matches) return;

	// ---- sections rise in as they scroll into view ----
	// Only what is below the fold at this moment is marked: nothing the reader
	// already sees ever blinks out, and without this script nothing is hidden.
	var SEL = 'main .section-head, main .split > *, main .points > li, main .cards > li, main .chips, main .os-list, main .feedback, main figure, main .dev, main .facts, main .badges, main .notice, main .catalog > li';
	if (IO) {
		var vh = window.innerHeight;
		var marked = [].filter.call(d.querySelectorAll(SEL), function (el) {
			if (el.closest('.hero, .page-head')) return false;
			if (el.parentElement.closest(SEL)) return false;
			return el.getBoundingClientRect().top > vh;
		});
		var io = new IntersectionObserver(function (entries) {
			var k = 0;
			entries.forEach(function (en) {
				if (!en.isIntersecting) return;
				var el = en.target;
				io.unobserve(el);
				el.style.setProperty('--rd', Math.min(k++, 5) * 70 + 'ms');
				el.classList.add('in');
				setTimeout(function () {
					el.classList.remove('reveal', 'in');
					el.style.removeProperty('--rd');
				}, 1200);
			});
		}, { rootMargin: '0px 0px -8% 0px' });
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
	[].forEach.call(d.querySelectorAll('.card, .dev-card, .shot'), function (el) {
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
