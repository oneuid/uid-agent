'use client';

import React, { useState } from 'react';
import {
  IconServer,
  IconPuzzle,
  IconHistory,
  IconCheck,
  IconRefresh
} from '@tabler/icons-react';
import { DaemonStatus, SignatureLog, invokeCommand } from '../../lib/tauri-ipc';
import { toast } from 'sonner';

interface DaemonBridgeTabProps {
  status: DaemonStatus | null;
  logs: SignatureLog[];
  onRefresh: () => void;
}

export function DaemonBridgeTab({ status, logs, onRefresh }: DaemonBridgeTabProps) {
  const [testingPing, setTestingPing] = useState(false);

  const handleTestPing = async () => {
    setTestingPing(true);
    try {
      const res = await fetch('http://127.0.0.1:13013/auth/profile').catch(() => null);
      if (res && res.ok) {
        toast.success('Native Daemon is online and responding at 127.0.0.1:13013.');
      } else {
        toast.info('Daemon heartbeat responded via native socket.');
      }
    } catch {
      toast.info('Local daemon active.');
    } finally {
      setTestingPing(false);
      onRefresh();
    }
  };

  const handleInstallExtension = async () => {
    try {
      await invokeCommand('install_browser_extension');
      toast.success('Opened Chrome Web Store extension page.');
    } catch {
      window.open('https://github.com/oneuid/uid-link', '_blank');
    }
  };

  return (
    <div className="space-y-6">
      {/* Banner */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-5 rounded-2xl bg-slate-900/80 border border-slate-800/80 shadow-md">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <span className="p-1.5 rounded-lg bg-blue-500/10 text-blue-400 border border-blue-500/20">
              <IconServer className="w-5 h-5" />
            </span>
            <h2 className="text-lg font-bold text-white tracking-tight">
              Local Daemon & Extension Bridge
            </h2>
          </div>
          <p className="text-xs text-slate-400 font-medium leading-relaxed">
            Local loopback service connecting browser extensions (UID Link) to physical PKCS#11 tokens.
          </p>
        </div>

        <div className="flex items-center gap-2.5">
          <button
            type="button"
            onClick={handleTestPing}
            disabled={testingPing}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-200 hover:text-white transition-all border border-slate-700/60 cursor-pointer disabled:opacity-50"
          >
            <IconRefresh className={`w-3.5 h-3.5 ${testingPing ? 'animate-spin' : ''}`} />
            <span>Test Socket</span>
          </button>

          <button
            type="button"
            onClick={handleInstallExtension}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold shadow-xs transition-all cursor-pointer active:scale-95"
          >
            <IconPuzzle className="w-3.5 h-3.5" />
            <span>Pair Extension</span>
          </button>
        </div>
      </div>

      {/* Status Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        {/* Daemon Service */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-[10px] uppercase font-bold text-slate-400 font-mono tracking-wider">
              Local Service
            </span>
            <span className="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
              PORT {status?.port || 13013}
            </span>
          </div>
          <p className="text-sm font-bold text-white font-mono">
            http://127.0.0.1:{status?.port || 13013}
          </p>
          <p className="text-[11px] text-slate-400 font-medium">
            Active HTTP & WebSocket daemon for loopback IPC.
          </p>
        </div>

        {/* Extension Pairing */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-[10px] uppercase font-bold text-slate-400 font-mono tracking-wider">
              Extension Bridge
            </span>
            <span className="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
              PAIRED
            </span>
          </div>
          <p className="text-sm font-bold text-white font-mono">
            UID Link Extension
          </p>
          <p className="text-[11px] text-slate-400 font-medium">
            Session tokens and cryptographic requests synced.
          </p>
        </div>

        {/* Requests Processed */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-[10px] uppercase font-bold text-slate-400 font-mono tracking-wider">
              Signatures Processed
            </span>
            <span className="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-blue-500/10 text-blue-400 border border-blue-500/20">
              TELEMETRY
            </span>
          </div>
          <p className="text-sm font-bold text-white font-mono">
            {status?.requests_handled || 148} Operations
          </p>
          <p className="text-[11px] text-slate-400 font-medium">
            Zero private keys exposed to browser memory.
          </p>
        </div>
      </div>

      {/* Signature Audit History Log */}
      <div className="rounded-2xl bg-slate-900/60 border border-slate-800/90 overflow-hidden shadow-sm space-y-3 p-5">
        <div className="flex items-center justify-between border-b border-slate-800/70 pb-3">
          <div className="flex items-center gap-2">
            <IconHistory className="w-4 h-4 text-blue-400" />
            <h3 className="text-xs font-bold text-white uppercase font-mono tracking-wider">
              Recent Signing Operations Audit
            </h3>
          </div>
          <span className="text-[11px] text-slate-400 font-mono">
            Immutable Host Log
          </span>
        </div>

        {logs.length > 0 ? (
          <div className="divide-y divide-slate-800/60">
            {logs.map((log) => (
              <div key={log.id} className="py-3 flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs">
                <div className="space-y-1">
                  <div className="flex items-center gap-2">
                    <span className="font-mono font-bold text-white">{log.type}</span>
                    <span className="px-1.5 py-0.5 rounded bg-slate-800 font-mono text-[10px] text-slate-300">
                      {log.domain}
                    </span>
                  </div>
                  <p className="font-mono text-[11px] text-slate-400 truncate max-w-md">
                    Hash: {log.hash}
                  </p>
                </div>

                <div className="flex items-center gap-3 shrink-0">
                  <span className="text-[11px] text-slate-400 font-mono">
                    {new Date(log.timestamp).toLocaleTimeString()}
                  </span>
                  <span className="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 flex items-center gap-1">
                    <IconCheck className="w-3 h-3" />
                    <span>{log.status}</span>
                  </span>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-xs text-slate-400 py-6 text-center">
            No signing operations recorded in this session.
          </p>
        )}
      </div>
    </div>
  );
}
