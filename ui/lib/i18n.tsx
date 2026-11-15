'use client';

import React, { useState, useEffect, createContext, useContext } from 'react';
import ar from '../messages/ar.json';
import de from '../messages/de.json';
import en from '../messages/en.json';
import es from '../messages/es.json';
import fr from '../messages/fr.json';
import it from '../messages/it.json';
import ja from '../messages/ja.json';
import ko from '../messages/ko.json';
import pt from '../messages/pt.json';
import ru from '../messages/ru.json';
import th from '../messages/th.json';
import vi from '../messages/vi.json';
import zhHans from '../messages/zh-hans.json';
import zhHant from '../messages/zh-hant.json';

const dictionaries: Record<string, Record<string, string>> = {
  ar: ar as Record<string, string>,
  de: de as Record<string, string>,
  en: en as Record<string, string>,
  es: es as Record<string, string>,
  fr: fr as Record<string, string>,
  it: it as Record<string, string>,
  ja: ja as Record<string, string>,
  ko: ko as Record<string, string>,
  pt: pt as Record<string, string>,
  ru: ru as Record<string, string>,
  th: th as Record<string, string>,
  vi: vi as Record<string, string>,
  'zh-hans': zhHans as Record<string, string>,
  'zh-hant': zhHant as Record<string, string>,
};

export const SUPPORTED_LOCALES = [
  { code: 'en', label: 'English' },
  { code: 'vi', label: 'Tiếng Việt' },
  { code: 'zh-hans', label: '简体中文' },
  { code: 'zh-hant', label: '繁體中文' },
  { code: 'ja', label: '日本語' },
  { code: 'ko', label: '한국어' },
  { code: 'fr', label: 'Français' },
  { code: 'de', label: 'Deutsch' },
  { code: 'es', label: 'Español' },
  { code: 'pt', label: 'Português' },
  { code: 'it', label: 'Italiano' },
  { code: 'ru', label: 'Русский' },
  { code: 'th', label: 'ไทย' },
  { code: 'ar', label: 'العربية' },
];

export function resolveLocale(raw?: string | null): string {
  if (!raw) return 'en';
  const norm = raw.trim().toLowerCase().replace('_', '-');
  if (norm === 'zh' || norm === 'zh-cn' || norm === 'zh-sg' || norm === 'zh-hans') {
    return 'zh-hans';
  }
  if (norm === 'zh-tw' || norm === 'zh-hk' || norm === 'zh-mo' || norm === 'zh-hant') {
    return 'zh-hant';
  }
  if (dictionaries[norm]) return norm;
  const base = norm.split('-')[0];
  if (base === 'zh') return 'zh-hans';
  if (dictionaries[base]) return base;
  return 'en';
}

interface I18nContextType {
  locale: string;
  setLocale: (locale: string) => void;
  t: (key: string, defaultVal?: string) => string;
}

const I18nContext = createContext<I18nContextType>({
  locale: 'en',
  setLocale: () => {},
  t: (k, d) => d || k,
});

export function I18nProvider({ children }: { children: React.ReactNode }) {
  const [locale, setLocaleState] = useState<string>(() => {
    if (typeof window !== 'undefined') {
      const saved = localStorage.getItem('uid_agent_locale');
      if (saved) {
        const resolved = resolveLocale(saved);
        if (resolved !== saved) {
          localStorage.setItem('uid_agent_locale', resolved);
        }
        return resolved;
      }
      return resolveLocale(navigator.language);
    }
    return 'en';
  });

  useEffect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.lang = locale;
      document.documentElement.dir = locale === 'ar' ? 'rtl' : 'ltr';
    }
  }, [locale]);

  const setLocale = (next: string) => {
    const resolved = resolveLocale(next);
    setLocaleState(resolved);
    if (typeof window !== 'undefined') {
      localStorage.setItem('uid_agent_locale', resolved);
    }
  };

  const t = (key: string, defaultVal?: string): string => {
    const dict = dictionaries[locale] || dictionaries.en;
    if (dict && dict[key]) {
      return dict[key];
    }
    if (dictionaries.en && dictionaries.en[key]) {
      return dictionaries.en[key];
    }
    return defaultVal || key;
  };

  return (
    <I18nContext.Provider value={{ locale, setLocale, t }}>
      {children}
    </I18nContext.Provider>
  );
}

export function useI18n() {
  return useContext(I18nContext);
}
