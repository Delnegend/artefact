export function humanReadableSize(byte: number): string {
	if (Number.isNaN(byte)) {
		return '0 B'
	}
	const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB']
	let i = 0
	while (byte >= 1024) {
		// eslint-disable-next-line no-param-reassign
		byte /= 1024
		i++
	}
	return `${byte.toFixed(2)} ${units[i]}`
}
