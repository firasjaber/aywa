import { describe, expect, it } from 'vitest';
import { SETTINGS_TABS } from './settingsView';

describe('settings navigation', () => {
	it('includes Rules and exposes matching panel ids', () => {
		expect(SETTINGS_TABS).toEqual([
			{ id: 'app', label: 'App' },
			{ id: 'settings', label: 'Settings' },
			{ id: 'rules', label: 'Rules' },
			{ id: 'history', label: 'History' },
			{ id: 'debug', label: 'Debug' }
		]);
		expect(SETTINGS_TABS.map((tab) => `${tab.id}-panel`)).toEqual([
			'app-panel',
			'settings-panel',
			'rules-panel',
			'history-panel',
			'debug-panel'
		]);
	});
});
