import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { UpdateSnapshot } from './appUpdate';

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));
const snapshot: UpdateSnapshot = {
	revision: 0, currentVersion: '0.3.0', availableVersion: null, status: 'idle',
	downloadedBytes: 0, totalBytes: null, downloadReady: false, error: null
};
const flush = async () => { await new Promise((resolve) => setTimeout(resolve, 0)); };

beforeEach(() => { vi.resetModules(); vi.clearAllMocks(); });

describe('app update store', () => {
	it('keeps a newer event when the initial snapshot returns late, and cleans up', async () => {
		let onEvent!: (event: { payload: UpdateSnapshot }) => void;
		let resolveInitial!: (value: UpdateSnapshot) => void;
		const cleanup = vi.fn();
		mocks.listen.mockImplementation((_event, callback) => { onEvent = callback; return Promise.resolve(cleanup); });
		mocks.invoke.mockReturnValue(new Promise((resolve) => { resolveInitial = resolve; }));
		const { appUpdate } = await import('./appUpdate');
		let latest!: UpdateSnapshot;
		const unsubscribe = appUpdate.subscribe((value) => { latest = value; });
		await flush();
		onEvent({ payload: { ...snapshot, revision: 2, status: 'available', availableVersion: '0.4.0' } });
		resolveInitial(snapshot);
		await flush();
		expect(latest.availableVersion).toBe('0.4.0');
		unsubscribe();
		expect(cleanup).toHaveBeenCalledOnce();
	});

	it('preserves unknown download size and accepts a successful retry', async () => {
		let onEvent!: (event: { payload: UpdateSnapshot }) => void;
		mocks.listen.mockImplementation((_event, callback) => { onEvent = callback; return Promise.resolve(vi.fn()); });
		mocks.invoke.mockResolvedValue(snapshot);
		const { appUpdate, checkForUpdates } = await import('./appUpdate');
		let latest!: UpdateSnapshot;
		const unsubscribe = appUpdate.subscribe((value) => { latest = value; });
		await flush();
		onEvent({ payload: { ...snapshot, revision: 1, status: 'downloading', downloadedBytes: 150, totalBytes: null } });
		expect(latest.totalBytes).toBeNull();
		expect(latest.downloadedBytes).toBe(150);
		mocks.invoke.mockRejectedValueOnce(new Error('offline'));
		await expect(checkForUpdates()).rejects.toThrow('offline');
		mocks.invoke.mockResolvedValueOnce({ ...snapshot, revision: 2, status: 'upToDate' });
		await checkForUpdates();
		expect(latest.status).toBe('upToDate');
		unsubscribe();
	});

	it('cleans up a listener that resolves after unsubscribe', async () => {
		let resolveListen!: (cleanup: () => void) => void;
		mocks.listen.mockReturnValue(new Promise((resolve) => { resolveListen = resolve; }));
		const { appUpdate } = await import('./appUpdate');
		const unsubscribe = appUpdate.subscribe(() => {});
		unsubscribe();
		const cleanup = vi.fn();
		resolveListen(cleanup);
		await flush();
		expect(cleanup).toHaveBeenCalledOnce();
		expect(mocks.invoke).not.toHaveBeenCalled();
	});
});
