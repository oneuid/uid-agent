'use client';

import React from 'react';
import {
  IconShieldLock,
  IconRefresh,
  IconExternalLink,
  IconUser,
  IconLogout,
  IconLanguage
} from '@tabler/icons-react';
import { UserProfile, invokeCommand } from '../lib/tauri-ipc';
import { useI18n, SUPPORTED_LOCALES } from '../lib/i18n';
import { toast } from 'sonner';

interface HeaderProps {
  user: UserProfile | null;
  appVersion: string;
  onRefresh: () => void;
  onLogout: () => void;
}

export function Header({ user, appVersion, onRefresh, onLogout }: HeaderProps) {
  const { locale, setLocale, t } = useI18n();

  const handleOpenWeb = async () => {
    try {
      await invokeCommand('open_browser_url', { url: 'https://uid.one' });
    } catch {
      window.open('https://uid.one', '_blank');
    }
  };

  const handleConnect = async () => {
    try {
      await invokeCommand('open_login_window');
    } catch {
      handleOpenWeb();
    }
  };

  return (
    <header className="w-full bg-slate-900/90 border-b border-slate-800/80 px-5 py-3 flex items-center justify-between backdrop-blur-md sticky top-0 z-40">
      {/* Brand & Node Status */}
      <div className="flex items-center gap-3">
        <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-blue-600 to-indigo-700 flex items-center justify-center shadow-[0_2px_12px_rgba(37,99,235,0.4)]">
          <IconShieldLock className="w-5 h-5 text-white" />
        </div>
        <div>
          <div className="flex items-center gap-2">
            <span className="font-extrabold text-sm tracking-tight text-white">UID.one</span>
            <span className="px-1.5 py-0.5 rounded-full bg-blue-500/10 text-blue-400 border border-blue-500/20 text-[10px] font-mono font-bold">
              {appVersion}
            </span>
          </div>
          <div className="flex items-center gap-1.5 mt-0.5">
            <span className="relative flex h-2 w-2">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
              <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
            </span>
            <span className="text-[11px] font-medium text-emerald-400">
              {t('securedDevice', 'Secured Endpoint Node')}
            </span>
          </div>
        </div>
      </div>

      {/* Center / Right Actions: Language, Sync, User Profile */}
      <div className="flex items-center gap-2.5">
        {/* Language Selector Dropdown */}
        <div className="relative flex items-center">
          <div className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg bg-slate-800/80 border border-slate-700/60 text-xs text-slate-300">
            <IconLanguage className="w-3.5 h-3.5 text-slate-400" />
            <select
              value={locale}
              onChange={(e) => setLocale(e.target.value)}
              className="bg-transparent text-xs font-medium text-white focus:outline-none cursor-pointer pr-1"
            >
              {SUPPORTED_LOCALES.map((l) => (
                <option key={l.code} value={l.code} className="bg-slate-900 text-white">
                  {l.label}
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* Sync Button */}
        <button
          type="button"
          onClick={() => {
            onRefresh();
            toast.success(t('syncSuccess', 'Hardware bus and state synchronized.'));
          }}
          title="Refresh Hardware State"
          className="p-2 rounded-lg bg-slate-800/80 hover:bg-slate-700/80 border border-slate-700/60 text-slate-300 hover:text-white transition-colors cursor-pointer"
        >
          <IconRefresh className="w-4 h-4" />
        </button>

        {/* User Identity Profile / Connect Account */}
        {user ? (
          <div className="flex items-center gap-2 pl-2 border-l border-slate-800">
            <div className="w-7 h-7 rounded-full bg-blue-600/30 border border-blue-500/40 flex items-center justify-center text-blue-400 text-xs font-bold font-mono">
              {user.name ? user.name.charAt(0).toUpperCase() : 'U'}
            </div>
            <div className="hidden sm:block text-left">
              <p className="text-xs font-semibold text-white leading-tight truncate max-w-[130px]">
                {user.name || user.email}
              </p>
              <p className="text-[10px] text-slate-400 font-mono leading-tight truncate max-w-[130px]">
                {user.email}
              </p>
            </div>
            <button
              type="button"
              onClick={onLogout}
              title={t('userLogout', 'Log Out')}
              className="p-1.5 rounded-lg hover:bg-red-500/10 text-slate-400 hover:text-red-400 transition-colors cursor-pointer"
            >
              <IconLogout className="w-4 h-4" />
            </button>
          </div>
        ) : (
          <button
            type="button"
            onClick={handleConnect}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold shadow-xs transition-all cursor-pointer active:scale-95"
          >
            <IconUser className="w-3.5 h-3.5" />
            <span>{t('userConnectAccount', 'Connect Account')}</span>
          </button>
        )}

        {/* Open Web Portal Button */}
        <button
          type="button"
          onClick={handleOpenWeb}
          title="Open UID.one Web Portal"
          className="p-2 rounded-lg bg-slate-800/80 hover:bg-slate-700/80 border border-slate-700/60 text-slate-400 hover:text-white transition-colors cursor-pointer"
        >
          <IconExternalLink className="w-4 h-4" />
        </button>
      </div>
    </header>
  );
}
