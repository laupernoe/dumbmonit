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
