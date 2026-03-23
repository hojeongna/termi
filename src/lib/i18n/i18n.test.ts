import { describe, it, expect } from 'vitest';
import { EN as en } from '$lib/i18n/en';
import { KO as ko } from '$lib/i18n/ko';
import type { Locale, Translations } from '$lib/i18n/types';

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

// Collect all leaf keys from a translations object (including function keys)
function collectKeys(obj: unknown, prefix = ''): string[] {
  const keys: string[] = [];
  if (!isRecord(obj)) return keys;
  for (const key of Object.keys(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    const value = obj[key];
    if (isRecord(value)) {
      keys.push(...collectKeys(value, fullKey));
    } else {
      keys.push(fullKey);
    }
  }
  return keys;
}

describe('Translation files', () => {
  it('en and ko have the same set of keys', () => {
    const enKeys = collectKeys(en).sort();
    const koKeys = collectKeys(ko).sort();
    expect(enKeys).toEqual(koKeys);
  });

  it('en has no empty string values', () => {
    const keys = collectKeys(en);
    for (const key of keys) {
      const init: unknown = en;
      const value: unknown = key.split('.').reduce((obj: unknown, k) => isRecord(obj) ? obj[k] : undefined, init);
      if (typeof value === 'string') {
        expect(value.length, `en.${key} should not be empty`).toBeGreaterThan(0);
      }
    }
  });

  it('ko has no empty string values', () => {
    const keys = collectKeys(ko);
    for (const key of keys) {
      const init: unknown = ko;
      const value: unknown = key.split('.').reduce((obj: unknown, k) => isRecord(obj) ? obj[k] : undefined, init);
      if (typeof value === 'string') {
        expect(value.length, `ko.${key} should not be empty`).toBeGreaterThan(0);
      }
    }
  });

  it('function translations produce non-empty strings', () => {
    expect(en.projectItem.deleteConfirm('test').length).toBeGreaterThan(0);
    expect(ko.projectItem.deleteConfirm('test').length).toBeGreaterThan(0);
    expect(en.projectItem.runningCount(3).length).toBeGreaterThan(0);
    expect(ko.projectItem.runningCount(3).length).toBeGreaterThan(0);
    expect(en.settings.debug.count(5).length).toBeGreaterThan(0);
    expect(ko.settings.debug.count(5).length).toBeGreaterThan(0);
    expect(en.settings.theme.saveFailed('err').length).toBeGreaterThan(0);
    expect(ko.settings.theme.saveFailed('err').length).toBeGreaterThan(0);
    expect(en.settings.theme.deleteFailed('err').length).toBeGreaterThan(0);
    expect(ko.settings.theme.deleteFailed('err').length).toBeGreaterThan(0);
  });
});

describe('i18n store', () => {
  it('AVAILABLE_LOCALES contains en and ko', async () => {
    const { AVAILABLE_LOCALES } = await import('./index.svelte');
    expect(AVAILABLE_LOCALES).toHaveLength(2);
    const codes = AVAILABLE_LOCALES.map(l => l.code);
    expect(codes).toContain('en');
    expect(codes).toContain('ko');
  });

  it('getLocale returns default locale en', async () => {
    const { getLocale } = await import('./index.svelte');
    expect(getLocale()).toBe('en');
  });

  it('setLocale changes the current locale', async () => {
    const { setLocale, getLocale } = await import('./index.svelte');
    setLocale('ko');
    expect(getLocale()).toBe('ko');
    // Reset
    setLocale('en');
    expect(getLocale()).toBe('en');
  });

  it('getTranslations returns translations for the current locale', async () => {
    const { setLocale, getTranslations } = await import('./index.svelte');
    setLocale('en');
    expect(getTranslations().app.title).toBe('termi');
    expect(getTranslations().sidebar.addProject).toBe('Add Project');

    setLocale('ko');
    expect(getTranslations().app.title).toBe('termi');
    expect(getTranslations().sidebar.addProject).toBe('프로젝트 추가');

    // Reset
    setLocale('en');
  });
});
