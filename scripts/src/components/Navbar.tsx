import React from 'react';
import { ShieldCheck, Cpu, Terminal, BookOpen, Layers, Radio, User, ChevronDown } from 'lucide-react';
import { UserAccount } from '../types';
import { MOCK_USERS, TESTNET_CONTRACT_ID } from '../data/mockContractData';

interface NavbarProps {
  activeTab: string;
  setActiveTab: (tab: string) => void;
  currentUser: UserAccount;
  setCurrentUser: (user: UserAccount) => void;
  stats: { totalProjects: number; totalVerified: number; totalReviews: number };
}

export const Navbar: React.FC<NavbarProps> = ({
  activeTab,
  setActiveTab,
  currentUser,
  setCurrentUser,
  stats,
}) => {
  const [showUserDropdown, setShowUserDropdown] = React.useState(false);

  const tabs = [
    { id: 'registry', label: 'Project Registry', icon: Layers, count: stats.totalProjects },
    { id: 'collections', label: 'Collections', icon: ShieldCheck },
    { id: 'admin', label: 'Admin & Governance', icon: Cpu },
    { id: 'simulator', label: 'Soroban Invoker', icon: Terminal },
    { id: 'events', label: 'Event Log', icon: Radio },
    { id: 'docs', label: 'API Specs & Schema', icon: BookOpen },
  ];

  return (
    <header className="sticky top-0 z-40 bg-slate-900/90 backdrop-blur border-b border-slate-800">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          {/* Logo & Protocol Info */}
          <div className="flex items-center space-x-3 cursor-pointer" onClick={() => setActiveTab('registry')}>
            <div className="w-10 h-10 rounded-xl bg-gradient-to-tr from-blue-600 to-purple-600 p-0.5 shadow-lg shadow-blue-500/20">
              <div className="w-full h-full bg-slate-950 rounded-[10px] flex items-center justify-center">
                <Cpu className="w-5 h-5 text-blue-400" />
              </div>
            </div>
            <div>
              <div className="flex items-center space-x-2">
                <h1 className="font-bold text-lg text-white tracking-tight">Dongle Protocol</h1>
                <span className="text-[10px] uppercase font-mono px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-400 border border-blue-500/20">
                  Soroban Smart Contract
                </span>
              </div>
              <p className="text-xs text-slate-400 font-mono hidden sm:block">
                Contract: <span className="text-slate-300">{TESTNET_CONTRACT_ID.slice(0, 8)}...{TESTNET_CONTRACT_ID.slice(-6)}</span>
              </p>
            </div>
          </div>

          {/* Navigation Links */}
          <nav className="hidden lg:flex items-center space-x-1">
            {tabs.map((tab) => {
              const Icon = tab.icon;
              const isActive = activeTab === tab.id;
              return (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className={`flex items-center space-x-2 px-3 py-2 rounded-lg text-sm font-medium transition-all ${
                    isActive
                      ? 'bg-blue-600/15 text-blue-400 border border-blue-500/30'
                      : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
                  }`}
                >
                  <Icon className="w-4 h-4" />
                  <span>{tab.label}</span>
                  {tab.count !== undefined && (
                    <span className="ml-1 text-xs font-mono bg-slate-800 px-1.5 py-0.2 rounded text-slate-300">
                      {tab.count}
                    </span>
                  )}
                </button>
              );
            })}
          </nav>

          {/* Account Role Switcher & Network Status */}
          <div className="flex items-center space-x-3">
            <div className="hidden sm:flex items-center space-x-1.5 px-2.5 py-1 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-xs font-mono">
              <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
              <span>Stellar Testnet</span>
            </div>

            {/* Account Switcher Dropdown */}
            <div className="relative">
              <button
                onClick={() => setShowUserDropdown(!showUserDropdown)}
                className="flex items-center space-x-2 px-3 py-1.5 rounded-lg bg-slate-800 border border-slate-700 hover:border-slate-600 text-xs text-slate-200 transition"
              >
                <div className={`w-2.5 h-2.5 rounded-full ${currentUser.role === 'admin' ? 'bg-purple-400' : currentUser.role === 'owner' ? 'bg-blue-400' : 'bg-emerald-400'}`} />
                <div className="text-left">
                  <div className="font-semibold text-white leading-tight">{currentUser.name.split(' ')[0]}</div>
                  <div className="text-[10px] text-slate-400 uppercase tracking-wider">{currentUser.role}</div>
                </div>
                <ChevronDown className="w-3.5 h-3.5 text-slate-400" />
              </button>

              {showUserDropdown && (
                <div className="absolute right-0 mt-2 w-64 bg-slate-800 border border-slate-700 rounded-xl shadow-xl z-50 py-1 divide-y divide-slate-700/50">
                  <div className="px-3 py-2 text-xs text-slate-400">
                    Switch Active Account Persona:
                  </div>
                  {MOCK_USERS.map((usr) => (
                    <button
                      key={usr.address}
                      onClick={() => {
                        setCurrentUser(usr);
                        setShowUserDropdown(false);
                      }}
                      className={`w-full text-left px-3 py-2.5 text-xs flex items-center justify-between hover:bg-slate-700/50 ${
                        currentUser.address === usr.address ? 'bg-blue-600/10 text-blue-400' : 'text-slate-300'
                      }`}
                    >
                      <div>
                        <div className="font-medium text-white">{usr.name}</div>
                        <div className="text-[10px] text-slate-400 font-mono">{usr.address.slice(0, 10)}...</div>
                      </div>
                      <span className="text-[10px] uppercase font-semibold px-1.5 py-0.5 rounded bg-slate-900 text-slate-300 border border-slate-700">
                        {usr.role}
                      </span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Mobile Nav Tabs */}
        <div className="lg:hidden flex items-center space-x-1 overflow-x-auto py-2 border-t border-slate-800 scrollbar-none">
          {tabs.map((tab) => {
            const Icon = tab.icon;
            const isActive = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`flex items-center space-x-1.5 whitespace-nowrap px-3 py-1.5 rounded-md text-xs font-medium ${
                  isActive ? 'bg-blue-600 text-white' : 'text-slate-400 bg-slate-800/40'
                }`}
              >
                <Icon className="w-3.5 h-3.5" />
                <span>{tab.label}</span>
              </button>
            );
          })}
        </div>
      </div>
    </header>
  );
};
