// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { DEFAULT_DARK, DEFAULT_LIGHT, mergeThemeColors, applyThemeToDocument } from './theme.svelte';

describe('DEFAULT_DARK', () => {
  it('has all required color keys', () => {
    const requiredKeys = [
      'bg-primary', 'bg-secondary', 'bg-surface',
      'text-primary', 'text-secondary',
      'accent', 'accent-hover', 'border',
      'danger', 'success',
      'status-working', 'status-waiting', 'status-completed',
    ];
    for (const key of requiredKeys) {
      expect(DEFAULT_DARK).toHaveProperty(key);
    }
  });

  it('has hex color values', () => {
    for (const value of Object.values(DEFAULT_DARK)) {
      expect(value).toMatch(/^#[0-9a-fA-F]{6}$/);
    }
  });
});

describe('DEFAULT_LIGHT', () => {
  it('has all required color keys', () => {
    const requiredKeys = [
      'bg-primary', 'bg-secondary', 'bg-surface',
      'text-primary', 'text-secondary',
      'accent', 'accent-hover', 'border',
      'danger', 'success',
      'status-working', 'status-waiting', 'status-completed',
    ];
    for (const key of requiredKeys) {
      expect(DEFAULT_LIGHT).toHaveProperty(key);
    }
  });

  it('differs from dark theme', () => {
    expect(DEFAULT_LIGHT['bg-primary']).not.toBe(DEFAULT_DARK['bg-primary']);
  });
});

describe('mergeThemeColors', () => {
  it('uses base dark colors when theme type is dark and no overrides', () => {
    const result = mergeThemeColors('dark', {});
    expect(result).toEqual(DEFAULT_DARK);
  });

  it('uses base light colors when theme type is light and no overrides', () => {
    const result = mergeThemeColors('light', {});
    expect(result).toEqual(DEFAULT_LIGHT);
  });

  it('overrides specific colors while keeping base values', () => {
    const result = mergeThemeColors('dark', { accent: '#ff0000' });
    expect(result['accent']).toBe('#ff0000');
    expect(result['bg-primary']).toBe(DEFAULT_DARK['bg-primary']);
  });
});

describe('applyThemeToDocument', () => {
  beforeEach(() => {
    // Reset inline styles
    document.documentElement.style.cssText = '';
  });

  it('sets CSS variables on document root', () => {
    const colors = { 'bg-primary': '#111111', accent: '#222222' };
    applyThemeToDocument(colors);
    expect(document.documentElement.style.getPropertyValue('--termi-bg-primary')).toBe('#111111');
    expect(document.documentElement.style.getPropertyValue('--termi-accent')).toBe('#222222');
  });

  it('adds --termi- prefix to all color keys', () => {
    applyThemeToDocument({ 'bg-primary': '#aabbcc' });
    expect(document.documentElement.style.getPropertyValue('--termi-bg-primary')).toBe('#aabbcc');
  });
});
