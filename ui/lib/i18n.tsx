'use client';

import React, { useState, createContext, useContext } from 'react';
import en from '../messages/en.json';
import vi from '../messages/vi.json';
import zh from '../messages/zh.json';
import ja from '../messages/ja.json';
import ko from '../messages/ko.json';
import fr from '../messages/fr.json';
import de from '../messages/de.json';
import es from '../messages/es.json';
import it from '../messages/it.json';
import ru from '../messages/ru.json';
import th from '../messages/th.json';
import ar from '../messages/ar.json';

const dictionaries: Record<string, Record<string, string>> = {
  en: en as Record<string, string>,
  vi: vi as Record<string, string>,
  zh: zh as Record<string, string>,
  ja: ja as Record<string, string>,
  ko: ko as Record<string, string>,
  fr: fr as Record<string, string>,
  de: de as Record<string, string>,
  es: es as Record<string, string>,
  it: it as Record<string, string>,
  ru: ru as Record<string, string>,
  th: th as Record<string, string>,
  ar: ar as Record<string, string>,
};

export const SUPPORTED_LOCALES = [
  { code: 'en', label: 'English' },
  { code: 'vi', label: 'Tiếng Việt' },
  { code: 'zh', label: '中文 (简体)' },
  { code: 'ja', label: '日本語' },
  { code: 'ko', label: '한국어' },
  { code: 'fr', label: 'Français' },
  { code: 'de', label: 'Deutsch' },
  { code: 'es', label: 'Español' },
  { code: 'it', label: 'Italiano' },
  { code: 'ru', label: 'Русский' },
  { code: 'th', label: 'ไทย' },
  { code: 'ar', label: 'العربية' },
];

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
      if (saved && dictionaries[saved]) return saved;
      const browserLang = navigator.language?.split('-')[0];
      if (browserLang && dictionaries[browserLang]) return browserLang;
    }
    return 'en';
  });

  const setLocale = (newLocale: string) => {
    if (dictionaries[newLocale]) {
      setLocaleState(newLocale);
      if (typeof window !== 'undefined') {
        localStorage.setItem('uid_agent_locale', newLocale);
      }
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
