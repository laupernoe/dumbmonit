<script lang="ts">
	/** A console state: its shape and its word, side by side. */
	import { STATUS_MARK, STATUS_TEXT, statusWord, type ConsoleStatus } from './status.js';

	interface Props {
		status: ConsoleStatus;
		/** Only the mark, for tight spots; the word is still read out. */
		bare?: boolean;
		class?: string;
	}
	let { status, bare = false, class: cls = '' }: Props = $props();
</script>

<span class={`inline-flex items-center gap-1.5 font-mono text-[0.625rem] font-semibold tracking-[0.04em] uppercase ${STATUS_TEXT[status]} ${cls}`}>
	<span aria-hidden="true" class={`inline-block size-[7px] shrink-0 ${STATUS_MARK[status]}`}></span>
	<span class={bare ? 'sr-only' : ''}>{statusWord(status)}</span>
</span>
