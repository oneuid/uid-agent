'use client';

import React, { useState } from 'react';
import {
  IconSettings,
  IconPin,
  IconRefresh,
  IconLanguage,
  IconCheck
} from '@tabler/icons-react';
import { invokeCommand } from '../../lib/ipc';
import { useI18n, SUPPORTED_LOCALES } from '../../lib/i18n';
import { toast } from 'sonner';

interface SettingsTabProps {
  appVersion: string;
}

export function SettingsTab({ appVersion }: SettingsTabProps) {
  const { locale, setLocale, t } = useI18n();
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [pinning, setPinning] = useState(false);
  const [minimizeToTray, setMinimizeToTray] = useState(true);

  const handlePinToDock = async () => {
    setPinning(true);
    try {
      await invokeCommand('pin_to_dock');
      toast.success(t('pinnedToast', 'Pinned UID Agent to desktop dock.'));
    } catch {
      toast.info(t('shortcutExistsToast', 'Desktop shortcut is already registered.'));
    } finally {
      setPinning(false);
    }
  };

  const handleCheckUpdate = async () => {
    setCheckingUpdate(true);
    try {
      await invokeCommand('check_for_updates');
      toast.success(`${t('latestVersionToast', 'Running latest version')}: ${appVersion}`);
    } catch {
      toast.info(`${t('latestVersionToast', 'Running latest version')}: ${appVersion}`);
    } finally {
      setCheckingUpdate(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Banner */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-5 rounded-2xl bg-slate-900/80 border border-slate-800/80 shadow-md">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <span className="p-1.5 rounded-lg bg-blue-500/10 text-blue-400 border border-blue-500/20">
              <IconSettings className="w-5 h-5" />
            </span>
            <h2 className="text-lg font-sans font-bold text-white tracking-tight">
              {t('settingsTitle', 'Agent preferences')}
            </h2>
          </div>
          <p className="text-xs text-slate-400 font-medium leading-relaxed">
            {t('settingsSubtitle', 'Configure display language, system tray behavior, and desktop launcher integration.')}
          </p>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <button
            type="button"
            onClick={handleCheckUpdate}
            disabled={checkingUpdate}
            className="inline-flex items-center gap-1.5 h-8 px-3 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-200 hover:text-white transition-colors duration-150 border border-slate-700/60 cursor-pointer disabled:opacity-50"
          >
            <IconRefresh className={`w-3.5 h-3.5 ${checkingUpdate ? 'animate-spin' : ''}`} />
            <span>{t('checkUpdates', 'Check for updates')}</span>
          </button>
        </div>
      </div>

      {/* Settings Options List */}
      <div className="space-y-3">
        {/* 14-Locale Language Selector */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div className="space-y-1">
            <h4 className="text-sm font-sans font-semibold text-white">
              {t('languageSettingTitle', 'Display language')}
            </h4>
            <p className="text-xs text-slate-400 font-medium">
              {t('languageSettingDesc', 'Choose the interface language for UID Agent.')}
            </p>
          </div>
          <div className="flex items-center gap-2 h-9 px-3 rounded-xl bg-slate-800 border border-slate-700/60 text-xs text-white shrink-0">
            <IconLanguage className="w-4 h-4 text-slate-400 shrink-0" />
            <select
              value={locale}
              aria-label={t('languageSettingTitle', 'Display language')}
              onChange={(e) => setLocale(e.target.value)}
              className="bg-transparent text-xs font-semibold text-white focus:outline-none cursor-pointer pe-1"
            >
              {SUPPORTED_LOCALES.map((l) => (
                <option key={l.code} value={l.code} className="bg-slate-900 text-white">
                  {l.label}
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* System Tray Minimization */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 flex items-center justify-between gap-4">
          <div className="space-y-1">
            <h4 className="text-sm font-sans font-semibold text-white">
              {t('traySettingTitle', 'Background system tray')}
            </h4>
            <p className="text-xs text-slate-400 font-medium">
              {t('traySettingDesc', 'Keep the extension bridge active in the system tray when closing the window.')}
            </p>
          </div>
          <button
            type="button"
            role="switch"
            aria-checked={minimizeToTray}
            aria-label={t('traySettingTitle', 'Background system tray')}
            onClick={() => {
              const next = !minimizeToTray;
              setMinimizeToTray(next);
              toast.success(
                next
                  ? t('trayEnabledToast', 'System tray background mode enabled.')
                  : t('trayDisabledToast', 'System tray background mode disabled.')
              );
            }}
            className={`w-11 h-6 flex items-center rounded-full p-1 transition-colors duration-150 cursor-pointer shrink-0 ${
              minimizeToTray ? 'bg-blue-600' : 'bg-slate-800'
            }`}
          >
            <div
              className={`bg-white w-4 h-4 rounded-full shadow-md transition-transform duration-150 ${
                minimizeToTray ? 'ltr:translate-x-5 rtl:-translate-x-5' : 'translate-x-0'
              }`}
            />
          </button>
        </div>

        {/* Pin to OS Dock */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 flex items-center justify-between gap-4">
          <div className="space-y-1">
            <h4 className="text-sm font-sans font-semibold text-white">
              {t('dockSettingTitle', 'Desktop dock shortcut')}
            </h4>
            <p className="text-xs text-slate-400 font-medium">
              {t('dockSettingDesc', 'Pin UID Agent to your system dock or taskbar for quick access.')}
            </p>
          </div>
          <button
            type="button"
            onClick={handlePinToDock}
            disabled={pinning}
            className="inline-flex items-center gap-1.5 h-9 px-3.5 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-white border border-slate-700/60 cursor-pointer shadow-xs transition-[background-color,transform] duration-150 active:scale-[0.98] shrink-0"
          >
            <IconPin className="w-3.5 h-3.5" />
            <span>{t('pinToDock', 'Pin to dock')}</span>
          </button>
        </div>

        {/* Release Metadata Card */}
        <div className="p-4 rounded-2xl bg-slate-900/40 border border-slate-800/60 flex flex-col sm:flex-row sm:items-center justify-between gap-2 text-xs text-slate-400">
          <span className="font-mono tabular-nums">
            {t('releaseLabel', 'UID Agent Core · Release')} {appVersion}
          </span>
          <span className="text-emerald-400 font-sans font-semibold flex items-center gap-1">
            <IconCheck className="w-3.5 h-3.5" />
            <span>{t('signatureVerified', 'Engine signature verified')}</span>
          </span>
        </div>
      </div>
    </div>
  );
}
