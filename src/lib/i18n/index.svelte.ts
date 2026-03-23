import { EN } from './en';
import { KO } from './ko';
import type { Locale, Translations } from './types';

const LOCALES: Record<Locale, Translations> = { en: EN, ko: KO };

let currentLocale = $state<Locale>('en');

const _t: Translations = $derived(LOCALES[currentLocale]);

export const i18n = {
  get t(): Translations { return _t; },
  get locale(): Locale { return currentLocale; },
};

export function setLocale(locale: Locale): void {
  currentLocale = locale;
}

export function getLocale(): Locale {
  return currentLocale;
}

export function getTranslations(): Translations {
  return LOCALES[currentLocale];
}

export const AVAILABLE_LOCALES: { code: Locale; name: string }[] = [
  { code: 'en', name: 'English' },
  { code: 'ko', name: '한국어' },
];
