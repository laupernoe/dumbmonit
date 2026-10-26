/**
 * A deliberately small Markdown reader for text widgets.
 *
 * It produces a tree the component renders with plain elements — never HTML
 * strings — so nothing typed into a widget can inject markup. Supported:
 * headings (#, ##, ###), paragraphs, bullet and numbered lists, fenced code,
 * **bold**, *italic*, `code` and [links](https://…) (http, https, mailto and
 * site-relative only).
 */

export type Inline =
	| { t: 'text'; v: string }
	| { t: 'b' | 'i' | 'code'; v: string }
	| { t: 'a'; v: string; href: string };

export type Block =
	| { t: 'h'; level: 1 | 2 | 3; inline: Inline[] }
	| { t: 'p'; inline: Inline[] }
	| { t: 'ul' | 'ol'; items: Inline[][] }
	| { t: 'code'; text: string };

export function safeHref(href: string): string | null {
	const value = href.trim();
	if (/^(https?:\/\/|mailto:)/i.test(value)) return value;
	if (value.startsWith('/') && !value.startsWith('//')) return value;
	return null;
}

const TOKEN = /(\*\*([^*]+)\*\*|\*([^*]+)\*|`([^`]+)`|\[([^\]]+)\]\(([^)\s]+)\))/g;

export function parseInline(source: string): Inline[] {
	const out: Inline[] = [];
	let last = 0;
	for (const match of source.matchAll(TOKEN)) {
		const at = match.index ?? 0;
		if (at > last) out.push({ t: 'text', v: source.slice(last, at) });
		if (match[2] !== undefined) out.push({ t: 'b', v: match[2] });
		else if (match[3] !== undefined) out.push({ t: 'i', v: match[3] });
		else if (match[4] !== undefined) out.push({ t: 'code', v: match[4] });
		else {
			const href = safeHref(match[6]);
			if (href) out.push({ t: 'a', v: match[5], href });
			else out.push({ t: 'text', v: match[0] });
		}
		last = at + match[0].length;
	}
	if (last < source.length) out.push({ t: 'text', v: source.slice(last) });
	return out;
}

export function parseMarkdown(source: string): Block[] {
	const blocks: Block[] = [];
	const lines = source.replace(/\r\n?/g, '\n').split('\n');
	let paragraph: string[] = [];
	const flush = () => {
		if (paragraph.length) blocks.push({ t: 'p', inline: parseInline(paragraph.join(' ')) });
		paragraph = [];
	};
	for (let i = 0; i < lines.length; i += 1) {
		const line = lines[i];
		if (line.trimStart().startsWith('```')) {
			flush();
			const body: string[] = [];
			i += 1;
			while (i < lines.length && !lines[i].trimStart().startsWith('```')) body.push(lines[i++]);
			blocks.push({ t: 'code', text: body.join('\n') });
			continue;
		}
		const heading = /^(#{1,3})\s+(.*)$/.exec(line);
		if (heading) {
			flush();
			blocks.push({ t: 'h', level: heading[1].length as 1 | 2 | 3, inline: parseInline(heading[2]) });
			continue;
		}
		const item = /^\s*([-*]|\d+\.)\s+(.*)$/.exec(line);
		if (item) {
			flush();
			const kind = item[1].endsWith('.') ? 'ol' : 'ul';
			const previous = blocks[blocks.length - 1];
			const text = parseInline(item[2]);
			if (previous && previous.t === kind) previous.items.push(text);
			else blocks.push({ t: kind, items: [text] });
			continue;
		}
		if (line.trim() === '') flush();
		else paragraph.push(line.trim());
	}
	flush();
	return blocks;
}
