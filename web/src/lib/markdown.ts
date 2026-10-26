/**
 * Safe Markdown for runbooks. Every character of the source is HTML-escaped
 * first; only then is a small subset of Markdown turned into tags this module
 * writes itself: headings, lists, paragraphs, fenced code, inline code, bold
 * and links. No raw HTML ever passes through, and a link is only made for
 * http(s) or same-site paths (never `javascript:`, `data:` or `//host`).
 * No imports: the module runs as-is under `node --test`.
 */

export function escapeHtml(text: string): string {
	return text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		.replace(/'/g, '&#39;');
}

/** Only http(s) URLs and same-site absolute paths become links. */
export function safeHref(url: string): boolean {
	return /^https?:\/\/[^\s]+$/i.test(url) || /^\/(?!\/)[^\s]*$/.test(url);
}

const PLACEHOLDER = '\u0000';

/** Inline formatting on text that is already escaped. */
function inline(escaped: string): string {
	const codes: string[] = [];
	let out = escaped.replace(/`([^`]+)`/g, (_m, code: string) => {
		codes.push(code);
		return `${PLACEHOLDER}${codes.length - 1}${PLACEHOLDER}`;
	});
	out = out.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (whole, label: string, url: string) => {
		// The URL is escaped text: decode the ampersands only to test it.
		const raw = url.replace(/&amp;/g, '&');
		if (!safeHref(raw) || /&(quot|#39|lt|gt);/.test(url)) return whole;
		return `<a href="${url}" target="_blank" rel="noopener noreferrer nofollow">${label}</a>`;
	});
	out = out.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
	return out.replace(new RegExp(`${PLACEHOLDER}(\\d+)${PLACEHOLDER}`, 'g'), (_m, i: string) => `<code>${codes[Number(i)]}</code>`);
}

/** Renders Markdown to HTML that contains only the tags written here. */
export function renderMarkdown(source: string): string {
	const lines = source.replace(/\u0000/g, '').replace(/\r\n?/g, '\n').split('\n');
	const html: string[] = [];
	let paragraph: string[] = [];
	let list: { ordered: boolean; items: string[] } | null = null;
	let fence: string[] | null = null;

	const flushParagraph = () => {
		if (paragraph.length) html.push(`<p>${paragraph.map((line) => inline(escapeHtml(line))).join('<br>')}</p>`);
		paragraph = [];
	};
	const flushList = () => {
		if (list) {
			const tag = list.ordered ? 'ol' : 'ul';
			html.push(`<${tag}>${list.items.map((item) => `<li>${inline(escapeHtml(item))}</li>`).join('')}</${tag}>`);
		}
		list = null;
	};

	for (const line of lines) {
		if (fence) {
			if (/^\s*```/.test(line)) {
				html.push(`<pre><code>${escapeHtml(fence.join('\n'))}</code></pre>`);
				fence = null;
			} else {
				fence.push(line);
			}
			continue;
		}
		if (/^\s*```/.test(line)) {
			flushParagraph();
			flushList();
			fence = [];
			continue;
		}
		const heading = /^(#{1,3})\s+(.*)$/.exec(line);
		if (heading) {
			flushParagraph();
			flushList();
			const level = heading[1].length + 2; // h3..h5: below the page's own headings
			html.push(`<h${level}>${inline(escapeHtml(heading[2].trim()))}</h${level}>`);
			continue;
		}
		const bullet = /^\s*[-*]\s+(.*)$/.exec(line);
		const numbered = /^\s*\d+[.)]\s+(.*)$/.exec(line);
		if (bullet || numbered) {
			flushParagraph();
			const ordered = !!numbered;
			if (list && list.ordered !== ordered) flushList();
			if (!list) list = { ordered, items: [] };
			list.items.push((bullet ?? numbered)![1]);
			continue;
		}
		if (line.trim() === '') {
			flushParagraph();
			flushList();
			continue;
		}
		flushList();
		paragraph.push(line);
	}
	if (fence) html.push(`<pre><code>${escapeHtml(fence.join('\n'))}</code></pre>`);
	flushParagraph();
	flushList();
	return html.join('');
}
