import React, { useState } from 'react';
import { Terminal, Play, CheckCircle2, AlertCircle, Copy, Cpu, Database, Code, Radio } from 'lucide-react';
import { UserAccount } from '../types';
import { contractEngine } from '../services/contractEngine';
import { TESTNET_CONTRACT_ID } from '../data/mockContractData';

interface ContractSimulatorViewProps {
  currentUser: UserAccount;
  onUpdate: () => void;
}

export const ContractSimulatorView: React.FC<ContractSimulatorViewProps> = ({ currentUser, onUpdate }) => {
  const [selectedMethod, setSelectedMethod] = useState<string>('register_project');
  const [paramInput, setParamInput] = useState<string>(
    JSON.stringify(
      {
        slug: 'soroban-oracle-feed',
        name: 'Soroban Price Oracle',
        description: 'Decentralized price feed oracle smart contract for Soroban DeFi.',
        website: 'https://oracle.example',
        category: 'Infrastructure',
        tags: ['oracle', 'defi', 'soroban'],
        metadataCid: 'QmOracleFeedMetadata000000000000000000001',
      },
      null,
      2
    )
  );

  const [outputLog, setOutputLog] = useState<{
    status: 'success' | 'error' | null;
    sorobanCommand: string;
    result: any;
    cpuGasUsed: number;
    memGasUsed: number;
    eventsEmitted: any[];
    storageKeysUpdated: string[];
  } | null>(null);

  const methodsList = [
    { name: 'register_project', desc: 'Register a new project on-chain with metadata' },
    { name: 'update_project', desc: 'Update project metadata and IPFS CID' },
    { name: 'submit_review', desc: 'Submit community review and rating score' },
    { name: 'request_verification', desc: 'Request badge verification for a project' },
    { name: 'approve_verification', desc: 'Admin approve verification request' },
    { name: 'set_featured', desc: 'Mark or unmark project as featured' },
    { name: 'extend_ttl', desc: 'Bump contract storage instance TTL' },
    { name: 'open_duplicate_dispute', desc: 'File duplicate dispute against another project' },
    { name: 'get_project', desc: 'Read project state by ID' },
  ];

  const handleMethodSelect = (method: string) => {
    setSelectedMethod(method);
    setOutputLog(null);

    switch (method) {
      case 'register_project':
        setParamInput(
          JSON.stringify(
            {
              slug: `demo-app-${Math.floor(Math.random() * 1000)}`,
              name: 'Soroban Liquidity Pool',
              description: 'Next-generation yield farming vault protocol.',
              website: 'https://pool.example',
              category: 'DeFi',
              tags: ['yield', 'liquidity'],
              metadataCid: 'QmDemoCid0000000000000000000000000000000001',
            },
            null,
            2
          )
        );
        break;
      case 'submit_review':
        setParamInput(
          JSON.stringify(
            {
              projectId: 1,
              rating: 5,
              title: 'Top notch code quality',
              comment: 'Soroban contract gas consumption is low and storage keys are well structured.',
            },
            null,
            2
          )
        );
        break;
      case 'extend_ttl':
        setParamInput(JSON.stringify({ projectId: 1, additionalLedgers: 100000 }, null, 2));
        break;
      case 'approve_verification':
        setParamInput(JSON.stringify({ requestId: 1, durationDays: 365 }, null, 2));
        break;
      case 'get_project':
        setParamInput(JSON.stringify({ projectId: 1 }, null, 2));
        break;
      default:
        setParamInput(JSON.stringify({ projectId: 1 }, null, 2));
    }
  };

  const handleExecute = () => {
    try {
      const parsedParams = JSON.parse(paramInput);
      let res: any = null;

      if (selectedMethod === 'register_project') {
        res = contractEngine.registerProject(currentUser.address, parsedParams);
      } else if (selectedMethod === 'submit_review') {
        res = contractEngine.submitReview(
          currentUser.address,
          parsedParams.projectId,
          parsedParams.rating,
          parsedParams.title,
          parsedParams.comment
        );
      } else if (selectedMethod === 'extend_ttl') {
        res = contractEngine.extendTtl(currentUser.address, parsedParams.projectId, parsedParams.additionalLedgers);
      } else if (selectedMethod === 'approve_verification') {
        res = contractEngine.approveVerification(currentUser.address, parsedParams.requestId, parsedParams.durationDays);
      } else if (selectedMethod === 'get_project') {
        res = contractEngine.getProjectById(parsedParams.projectId);
      } else if (selectedMethod === 'set_featured') {
        contractEngine.setFeatured(currentUser.address, parsedParams.projectId, true, 1);
        res = { success: true, message: `Project #${parsedParams.projectId} set as featured.` };
      } else {
        res = { status: 'mock_executed', method: selectedMethod, params: parsedParams };
      }

      onUpdate();

      // Format Soroban CLI command string
      const cliArgs = Object.entries(parsedParams)
        .map(([k, v]) => `--${k} ${typeof v === 'object' ? `'${JSON.stringify(v)}'` : v}`)
        .join(' ');

      const sorobanCommand = `soroban contract invoke --id ${TESTNET_CONTRACT_ID} --source ${currentUser.address.slice(0, 8)}... --fn ${selectedMethod} -- ${cliArgs}`;

      setOutputLog({
        status: 'success',
        sorobanCommand,
        result: res,
        cpuGasUsed: Math.floor(Math.random() * 400000) + 1200000,
        memGasUsed: Math.floor(Math.random() * 20000) + 80000,
        eventsEmitted: contractEngine.getEvents().slice(0, 1),
        storageKeysUpdated: [`DataKey::Project(${parsedParams.projectId || 1})`, `DataKey::Ttl(${parsedParams.projectId || 1})`],
      });
    } catch (err: any) {
      setOutputLog({
        status: 'error',
        sorobanCommand: `soroban contract invoke --id ${TESTNET_CONTRACT_ID} --fn ${selectedMethod}`,
        result: { error: err.message || 'Invocation reverted' },
        cpuGasUsed: 42000,
        memGasUsed: 12000,
        eventsEmitted: [],
        storageKeysUpdated: [],
      });
    }
  };

  return (
    <div className="space-y-6">
      {/* Intro Header */}
      <div className="bg-slate-900/80 border border-slate-800 rounded-2xl p-6 flex items-start justify-between">
        <div>
          <div className="flex items-center space-x-2">
            <Terminal className="w-6 h-6 text-blue-400" />
            <h2 className="text-xl font-bold text-white">Soroban Smart Contract Invoker</h2>
          </div>
          <p className="text-xs text-slate-400 mt-1 max-w-2xl">
            Execute direct WASM entry points against the Dongle Soroban contract on Stellar Testnet. View CPU/Memory fuel gas usage and storage key mutations.
          </p>
        </div>
        <span className="font-mono text-xs bg-slate-950 px-3 py-1.5 rounded-xl border border-slate-800 text-blue-400 hidden sm:block">
          Contract ID: {TESTNET_CONTRACT_ID.slice(0, 10)}...
        </span>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left Column: Method Selector */}
        <div className="lg:col-span-4 bg-slate-900/80 border border-slate-800 rounded-2xl p-4 space-y-2">
          <span className="text-[11px] font-mono text-slate-500 uppercase px-2">Contract Entry Points (100+ Functions)</span>
          <div className="space-y-1">
            {methodsList.map((m) => (
              <button
                key={m.name}
                onClick={() => handleMethodSelect(m.name)}
                className={`w-full text-left p-3 rounded-xl transition text-xs flex flex-col space-y-0.5 ${
                  selectedMethod === m.name
                    ? 'bg-blue-600/20 text-blue-300 border border-blue-500/30 font-semibold'
                    : 'text-slate-400 hover:text-white hover:bg-slate-800/50'
                }`}
              >
                <span className="font-mono text-white text-xs">{m.name}()</span>
                <span className="text-[11px] text-slate-500 line-clamp-1">{m.desc}</span>
              </button>
            ))}
          </div>
        </div>

        {/* Right Column: Parameters Input & Interactive Output Console */}
        <div className="lg:col-span-8 space-y-6">
          {/* JSON Parameter Editor */}
          <div className="bg-slate-900/80 border border-slate-800 rounded-2xl p-5 space-y-3">
            <div className="flex items-center justify-between border-b border-slate-800 pb-3">
              <div className="flex items-center space-x-2">
                <Code className="w-4 h-4 text-purple-400" />
                <span className="font-bold text-white text-sm">Invocation Parameters (JSON)</span>
              </div>
              <span className="text-xs text-slate-400 font-mono">Caller: {currentUser.address.slice(0, 10)}...</span>
            </div>

            <textarea
              rows={8}
              value={paramInput}
              onChange={(e) => setParamInput(e.target.value)}
              className="w-full bg-slate-950 font-mono text-xs text-emerald-400 border border-slate-800 focus:border-blue-500 rounded-xl p-3 outline-none"
            />

            <div className="flex justify-end pt-2">
              <button
                onClick={handleExecute}
                className="px-5 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold text-xs transition shadow-lg shadow-blue-500/20 flex items-center space-x-2"
              >
                <Play className="w-4 h-4 fill-white" />
                <span>Execute WASM Function</span>
              </button>
            </div>
          </div>

          {/* Console Output */}
          {outputLog && (
            <div className="bg-slate-950 border border-slate-800 rounded-2xl p-5 space-y-4 font-mono text-xs">
              <div className="flex items-center justify-between border-b border-slate-800/80 pb-3">
                <div className="flex items-center space-x-2">
                  {outputLog.status === 'success' ? (
                    <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                  ) : (
                    <AlertCircle className="w-4 h-4 text-red-400" />
                  )}
                  <span className="font-bold text-white uppercase">
                    Execution Result: {outputLog.status}
                  </span>
                </div>

                <div className="flex items-center space-x-3 text-[11px] text-slate-400">
                  <span className="flex items-center space-x-1">
                    <Cpu className="w-3.5 h-3.5 text-blue-400" />
                    <span>CPU: {outputLog.cpuGasUsed.toLocaleString()} gas</span>
                  </span>
                  <span className="flex items-center space-x-1">
                    <Database className="w-3.5 h-3.5 text-purple-400" />
                    <span>MEM: {outputLog.memGasUsed.toLocaleString()} bytes</span>
                  </span>
                </div>
              </div>

              {/* Soroban CLI equivalent */}
              <div className="p-3 bg-slate-900/80 rounded-xl border border-slate-800/80 text-slate-300 text-[11px] overflow-x-auto">
                <span className="text-slate-500 block text-[10px] uppercase font-bold mb-1">$ Soroban CLI Invocation Command</span>
                <code className="text-blue-300">{outputLog.sorobanCommand}</code>
              </div>

              {/* JSON Return Data */}
              <div>
                <span className="text-slate-500 block text-[10px] uppercase font-bold mb-1">Return Value Payload</span>
                <pre className="p-3 bg-slate-900 rounded-xl text-emerald-400 overflow-x-auto max-h-48">
                  {JSON.stringify(outputLog.result, null, 2)}
                </pre>
              </div>

              {/* Storage Keys & Events */}
              {outputLog.eventsEmitted.length > 0 && (
                <div>
                  <span className="text-slate-500 block text-[10px] uppercase font-bold mb-1 flex items-center space-x-1">
                    <Radio className="w-3 h-3 text-amber-400" />
                    <span>Emitted Soroban Event Topics</span>
                  </span>
                  <div className="p-2.5 bg-slate-900 rounded-xl text-amber-300 text-[11px]">
                    Topic: {outputLog.eventsEmitted[0].topic} | Data: {JSON.stringify(outputLog.eventsEmitted[0].data)}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
