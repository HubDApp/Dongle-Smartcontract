import React, { useState } from 'react';
import { BookOpen, FileText, Database, ShieldAlert, Radio, Code } from 'lucide-react';

export const ContractDocsView: React.FC = () => {
  const [docTab, setDocTab] = useState<'interface' | 'storage' | 'events' | 'threats'>('interface');

  return (
    <div className="space-y-6">
      {/* Docs Header */}
      <div className="bg-slate-900/80 border border-slate-800 rounded-2xl p-6 flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
        <div>
          <div className="flex items-center space-x-2">
            <BookOpen className="w-6 h-6 text-blue-400" />
            <h2 className="text-xl font-bold text-white">Dongle Protocol Specifications & Specs</h2>
          </div>
          <p className="text-xs text-slate-400 mt-1">
            Complete technical specification documentation for Soroban smart contract functions, storage schema, events, and threat model.
          </p>
        </div>

        <div className="flex bg-slate-950 p-1 rounded-xl border border-slate-800 text-xs font-medium">
          <button
            onClick={() => setDocTab('interface')}
            className={`px-3 py-1.5 rounded-lg transition ${docTab === 'interface' ? 'bg-blue-600 text-white' : 'text-slate-400'}`}
          >
            Contract Functions
          </button>
          <button
            onClick={() => setDocTab('storage')}
            className={`px-3 py-1.5 rounded-lg transition ${docTab === 'storage' ? 'bg-blue-600 text-white' : 'text-slate-400'}`}
          >
            Storage Schema
          </button>
          <button
            onClick={() => setDocTab('events')}
            className={`px-3 py-1.5 rounded-lg transition ${docTab === 'events' ? 'bg-blue-600 text-white' : 'text-slate-400'}`}
          >
            Event Schema
          </button>
          <button
            onClick={() => setDocTab('threats')}
            className={`px-3 py-1.5 rounded-lg transition ${docTab === 'threats' ? 'bg-blue-600 text-white' : 'text-slate-400'}`}
          >
            Threat Model
          </button>
        </div>
      </div>

      {/* Tab Content */}
      <div className="bg-slate-900/80 border border-slate-800 rounded-2xl p-6 text-sm text-slate-300 space-y-4">
        {docTab === 'interface' && (
          <div className="space-y-6">
            <h3 className="text-lg font-bold text-white flex items-center space-x-2 border-b border-slate-800 pb-2">
              <Code className="w-5 h-5 text-blue-400" />
              <span>Smart Contract Function Specification (Soroban SDK 20)</span>
            </h3>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="p-4 bg-slate-950 border border-slate-800 rounded-xl space-y-2">
                <span className="font-mono text-blue-400 font-bold text-xs">register_project(env, owner, slug, name, desc, category)</span>
                <p className="text-xs text-slate-400">Registers a new project on-chain. Generates project ID, verifies slug uniqueness, charges registration fee if enabled.</p>
              </div>

              <div className="p-4 bg-slate-950 border border-slate-800 rounded-xl space-y-2">
                <span className="font-mono text-blue-400 font-bold text-xs">submit_review(env, reviewer, project_id, rating, title, comment)</span>
                <p className="text-xs text-slate-400">Submits review rating (1-5 stars) for a project. Enforces one review per reviewer per project and owner block.</p>
              </div>

              <div className="p-4 bg-slate-950 border border-slate-800 rounded-xl space-y-2">
                <span className="font-mono text-blue-400 font-bold text-xs">approve_verification(env, admin, request_id, duration_days)</span>
                <p className="text-xs text-slate-400">Admin method to approve verification badge level, issuing on-chain expiration timestamp.</p>
              </div>

              <div className="p-4 bg-slate-950 border border-slate-800 rounded-xl space-y-2">
                <span className="font-mono text-blue-400 font-bold text-xs">extend_ttl(env, caller, project_id, additional_ledgers)</span>
                <p className="text-xs text-slate-400">Extends Soroban instance and persistent storage footprint TTL ledgers to prevent data archival.</p>
              </div>
            </div>
          </div>
        )}

        {docTab === 'storage' && (
          <div className="space-y-4">
            <h3 className="text-lg font-bold text-white flex items-center space-x-2 border-b border-slate-800 pb-2">
              <Database className="w-5 h-5 text-purple-400" />
              <span>Soroban Storage Key Architecture</span>
            </h3>

            <div className="space-y-3 font-mono text-xs">
              <div className="p-3 bg-slate-950 border border-slate-800 rounded-xl">
                <span className="text-purple-400 font-bold">DataKey::Project(u64)</span> &rarr; Persistent storage of project metadata, owner, rating stats, badges.
              </div>

              <div className="p-3 bg-slate-950 border border-slate-800 rounded-xl">
                <span className="text-purple-400 font-bold">DataKey::SlugToId(Symbol)</span> &rarr; Fast lookup index mapping slug to project ID.
              </div>

              <div className="p-3 bg-slate-950 border border-slate-800 rounded-xl">
                <span className="text-purple-400 font-bold">DataKey::Review(u64, Address)</span> &rarr; Unique composite key mapping project ID and reviewer address to review object.
              </div>

              <div className="p-3 bg-slate-950 border border-slate-800 rounded-xl">
                <span className="text-purple-400 font-bold">DataKey::FeeConfig</span> &rarr; Instance storage holding XLM/SAC token contract address and operation fee amounts.
              </div>
            </div>
          </div>
        )}

        {docTab === 'events' && (
          <div className="space-y-4">
            <h3 className="text-lg font-bold text-white flex items-center space-x-2 border-b border-slate-800 pb-2">
              <Radio className="w-5 h-5 text-amber-400" />
              <span>Event Topics for Indexers</span>
            </h3>
            <ul className="list-disc list-inside space-y-2 text-xs text-slate-300">
              <li><strong className="font-mono text-amber-400">Dongle:register_project</strong>: Emitted when a project registration succeeds.</li>
              <li><strong className="font-mono text-amber-400">Dongle:submit_review</strong>: Emitted when a review is recorded and aggregated rating is updated.</li>
              <li><strong className="font-mono text-amber-400">Dongle:approve_verification</strong>: Emitted when an admin grants a verification badge.</li>
              <li><strong className="font-mono text-amber-400">Dongle:extend_ttl</strong>: Emitted when contract storage TTL footprint is extended.</li>
            </ul>
          </div>
        )}

        {docTab === 'threats' && (
          <div className="space-y-4">
            <h3 className="text-lg font-bold text-white flex items-center space-x-2 border-b border-slate-800 pb-2">
              <ShieldAlert className="w-5 h-5 text-red-400" />
              <span>Security Analysis & Mitigation Strategies</span>
            </h3>

            <div className="space-y-3 text-xs">
              <div className="p-3 bg-slate-950 border border-slate-800 rounded-xl">
                <span className="font-bold text-red-400">Sybil Review Manipulation:</span> Mitigated by requiring unique user addresses per review, owner self-review blocks, and fee configuration.
              </div>
              <div className="p-3 bg-slate-950 border border-slate-800 rounded-xl">
                <span className="font-bold text-red-400">Storage TTL Archival:</span> Mitigated by automatic bumpInstanceFootprint calls and manual `extend_ttl` helper methods.
              </div>
              <div className="p-3 bg-slate-950 border border-slate-800 rounded-xl">
                <span className="font-bold text-red-400">Admin Privilege Escalation:</span> Admin operations log actions to on-chain event logs and require superadmin consensus or timelocks.
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
