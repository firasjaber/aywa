import { describe, expect, it } from 'vitest';
import { formatHistoryDuration, formatRelativeHistoryTime, nextSelectedId } from './historyView';

describe('history view helpers', () => {
	const now = Date.UTC(2026, 7, 16, 12, 0, 0);

	it('formats relative times', () => {
		expect(formatRelativeHistoryTime(now - 20_000, now)).toBe('Just now');
		expect(formatRelativeHistoryTime(now - 3_600_000, now)).toBe('1h ago');
		expect(formatRelativeHistoryTime(now - 7 * 86_400_000, now)).toBe('1 week ago');
	});

	it('formats audio duration and selection replacement', () => {
		expect(formatHistoryDuration(65_000)).toBe('1:05');
		expect(formatHistoryDuration(3_661_000)).toBe('1:01:01');
		expect(nextSelectedId(['new', 'middle', 'old'], 'middle')).toBe('old');
		expect(nextSelectedId(['only'], 'only')).toBeNull();
	});
});
