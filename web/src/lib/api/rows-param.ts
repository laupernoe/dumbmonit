/** `rows=` value: no list means every row; an empty list must never mean that, so it names row 0. */
export function rowsParam(rows?: number[]): string | undefined {
	if (!rows) return undefined;
	return rows.length > 0 ? rows.join(',') : '0';
}
