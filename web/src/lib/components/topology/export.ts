/**
 * Export of the map as a standalone sheet: SVG, or PNG drawn from it.
 *
 * The sheet is the print sheet (`[data-topology-sheet]`, an SVG drawn by
 * `MapContent` at scale 1), cloned with every computed style written inline,
 * so the file renders the same anywhere — no stylesheet, no CSS variable.
 * Styles are read with the light theme forced for the duration of the clone:
 * an exported sheet is meant for paper, white ground and dark ink, whatever
 * the theme on screen. The toggle is synchronous, the screen never repaints
 * in between.
 *
 * The PNG is drawn from the SVG through an `<img>`. The image must come from a
 * `data:` URL, never a `blob:` one: the interface's Content-Security-Policy
 * (`img-src 'self' data:`) refuses blob images, which made the browser fire
 * `error` on the image and the export fail on every click.
 */

/** Why a local export failed (never a network problem: nothing is sent anywhere). */
export class ExportError extends Error {
	readonly step: 'image' | 'canvas' | 'png';
	constructor(step: 'image' | 'canvas' | 'png') {
		super(`map export failed at the ${step} step`);
		this.name = 'ExportError';
		this.step = step;
	}
}

const INLINE = [
	'fill',
	'fill-opacity',
	'stroke',
	'stroke-width',
	'stroke-dasharray',
	'stroke-linecap',
	'stroke-linejoin',
	'stroke-opacity',
	'paint-order',
	'opacity',
	'color',
	'font-family',
	'font-size',
	'font-weight',
	'letter-spacing',
	'text-anchor'
] as const;

const SVG_NS = 'http://www.w3.org/2000/svg';

function inlineStyles(source: Element, target: Element) {
	const computed = getComputedStyle(source);
	const style = INLINE.map((name) => `${name}:${computed.getPropertyValue(name)}`).join(';');
	target.setAttribute('style', style);
	target.removeAttribute('class');
	if (source instanceof SVGSVGElement && source.ownerSVGElement !== null) {
		// Nested icon: its size came from a class. Only a real length is kept
		// (`auto` is what a sheet that is not laid out reports).
		if (computed.width.endsWith('px')) target.setAttribute('width', computed.width);
		if (computed.height.endsWith('px')) target.setAttribute('height', computed.height);
	}
	const from = source.children;
	const to = target.children;
	for (let i = 0; i < from.length && i < to.length; i++) inlineStyles(from[i], to[i]);
}

/** Serialises the print sheet as a self-contained SVG document. */
export function sheetSvg(sheet: SVGSVGElement): string {
	const html = document.documentElement;
	const dark = html.classList.contains('dark');
	if (dark) html.classList.remove('dark');
	let clone: SVGSVGElement;
	try {
		clone = sheet.cloneNode(true) as SVGSVGElement;
		inlineStyles(sheet, clone);
	} finally {
		if (dark) html.classList.add('dark');
	}
	clone.setAttribute('xmlns', SVG_NS);
	clone.removeAttribute('style');
	clone.querySelectorAll('a').forEach((a) => a.removeAttribute('href'));
	const { w, h } = viewBoxSize(clone.getAttribute('viewBox'));
	clone.setAttribute('width', String(w));
	clone.setAttribute('height', String(h));
	clone.setAttribute('font-family', "'Bricolage Grotesque Variable', system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif");
	return `<?xml version="1.0" encoding="UTF-8"?>\n${xmlSafe(new XMLSerializer().serializeToString(clone))}`;
}

/**
 * Drops what XML 1.0 forbids (control characters, a lone surrogate left by a
 * name cut in the middle of an emoji): one such character makes the whole
 * document unreadable for the browser.
 */
export function xmlSafe(text: string): string {
	return text
		.replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F\uFFFE\uFFFF]/g, '')
		.replace(/[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/g, '\uFFFD');
}

/** The SVG as a `data:` URL, the only kind of image the page's policy lets through. */
export function svgDataUrl(svg: string): string {
	return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

/** Width and height from a `viewBox` attribute (0 when missing). */
export function viewBoxSize(viewBox: string | null): { w: number; h: number } {
	const [, , w, h] = (viewBox ?? '').trim().split(/[\s,]+/).map(Number);
	return { w: Number.isFinite(w) ? w : 0, h: Number.isFinite(h) ? h : 0 };
}

function download(blob: Blob, filename: string) {
	const url = URL.createObjectURL(blob);
	const a = document.createElement('a');
	a.href = url;
	a.download = filename;
	document.body.append(a);
	a.click();
	a.remove();
	setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function exportSvg(sheet: SVGSVGElement, filename: string) {
	download(new Blob([sheetSvg(sheet)], { type: 'image/svg+xml' }), filename);
}

/** Largest side of the PNG, in pixels: sharp on A3, still a reasonable file. */
const PNG_MAX = 6000;

/** Pixel size of the PNG: twice the sheet, within `PNG_MAX`. */
export function pngSize(w: number, h: number): { width: number; height: number } {
	const scale = Math.min(2, PNG_MAX / Math.max(w, h, 1));
	return { width: Math.max(1, Math.round(w * scale)), height: Math.max(1, Math.round(h * scale)) };
}

export async function exportPng(sheet: SVGSVGElement, filename: string): Promise<void> {
	const svg = sheetSvg(sheet);
	const { w, h } = viewBoxSize(sheet.getAttribute('viewBox'));
	const { width, height } = pngSize(w, h);
	const image = new Image();
	await new Promise<void>((resolve, reject) => {
		image.onload = () => resolve();
		image.onerror = () => reject(new ExportError('image'));
		image.src = svgDataUrl(svg);
	});
	const canvas = document.createElement('canvas');
	canvas.width = width;
	canvas.height = height;
	const context = canvas.getContext('2d');
	if (!context) throw new ExportError('canvas');
	context.fillStyle = '#ffffff';
	context.fillRect(0, 0, width, height);
	context.drawImage(image, 0, 0, width, height);
	const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
	if (!blob) throw new ExportError('png');
	download(blob, filename);
}
