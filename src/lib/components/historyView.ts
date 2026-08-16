export const formatHistoryDuration = (durationMs: number) => {
	const seconds = Math.max(0, Math.floor(durationMs / 1000));
	const hours = Math.floor(seconds / 3600);
	const minutes = Math.floor((seconds % 3600) / 60);
	const remainingSeconds = seconds % 60;
	const paddedSeconds = String(remainingSeconds).padStart(2, '0');
	return hours > 0 ? `${hours}:${String(minutes).padStart(2, '0')}:${paddedSeconds}` : `${minutes}:${paddedSeconds}`;
};

export const formatRelativeHistoryTime = (createdAtEpochMs: number, now = Date.now()) => {
	const elapsed = Math.max(0, now - createdAtEpochMs);
	const minute = 60_000;
	const hour = 60 * minute;
	const day = 24 * hour;
	if (elapsed < minute) return 'Just now';
	if (elapsed < hour) return `${Math.floor(elapsed / minute)}m ago`;
	if (elapsed < day) return `${Math.floor(elapsed / hour)}h ago`;
	if (elapsed < 7 * day) return `${Math.floor(elapsed / day)} day${Math.floor(elapsed / day) === 1 ? '' : 's'} ago`;
	if (elapsed < 30 * day) return `${Math.floor(elapsed / (7 * day))} week${Math.floor(elapsed / (7 * day)) === 1 ? '' : 's'} ago`;
	return new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric', year: 'numeric' }).format(createdAtEpochMs);
};

export const nextSelectedId = (ids: string[], deletedId: string) => {
	const index = ids.indexOf(deletedId);
	if (index < 0 || ids.length < 2) return null;
	return ids[index + 1] ?? ids[index - 1] ?? null;
};
