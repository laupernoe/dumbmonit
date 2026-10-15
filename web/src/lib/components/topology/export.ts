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
 */

const INLINE = [
	'fill',
	'fill-opacity',
	'stroke',
	'stroke-width',
	'stroke-dasharray',
	'stroke-linecap',
	'stroke-opacity',
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
		// Nested icon: its size came from a class.
		target.setAttribute('width', computed.width);
		target.setAttribute('height', computed.height);
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
	const [, , w, h] = (clone.getAttribute('viewBox') ?? '0 0 0 0').split(/\s+/).map(Number);
	clone.setAttribute('width', String(w));
	clone.setAttribute('height', String(h));
	clone.setAttribute('font-family', "'Bricolage Grotesque Variable', system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif");
	return `<?xml version="1.0" encoding="UTF-8"?>\n${new XMLSerializer().serializeToString(clone)}`;
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

export async function exportPng(sheet: SVGSVGElement, filename: string): Promise<void> {
	const svg = sheetSvg(sheet);
	const [, , w, h] = (sheet.getAttribute('viewBox') ?? '0 0 0 0').split(/\s+/).map(Number);
	const scale = Math.min(2, PNG_MAX / Math.max(w, h, 1));
	const url = URL.createObjectURL(new Blob([svg], { type: 'image/svg+xml' }));
	try {
		const image = new Image();
		await new Promise<void>((resolve, reject) => {
			image.onload = () => resolve();
			image.onerror = () => reject(new Error('image'));
			image.src = url;
		});
		const canvas = document.createElement('canvas');
		canvas.width = Math.round(w * scale);
		canvas.height = Math.round(h * scale);
		const context = canvas.getContext('2d');
		if (!context) throw new Error('canvas');
		context.fillStyle = '#ffffff';
		context.fillRect(0, 0, canvas.width, canvas.height);
		context.drawImage(image, 0, 0, canvas.width, canvas.height);
		const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
		if (!blob) throw new Error('png');
		download(blob, filename);
	} finally {
		URL.revokeObjectURL(url);
	}
}
