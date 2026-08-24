import React, { useState } from 'react';
import { Radio, Search, Filter, Layers, Clock, Terminal, ChevronRight } from 'lucide-react';
import { ContractEvent } from '../types';
import { contractEngine } from '../services/contractEngine';

export const EventStreamView: React.FC = () => {
  const [events, setEvents] = useState<ContractEvent[]>(contractEngine.getEvents());
  const [searchTopic, setSearchTopic] = useState('');

  const filteredEvents = events.filter((e) =>
    e.topic.toLowerCase().includes(searchTopic.toLowerCase()) ||
    JSON.stringify(e.data).toLowerCase().includes(searchTopic.toLowerCase())
  );

  return (
    <div className="space-y-6">
      <div className="bg-slate-900/80 border border-slate-800 rounded-2xl p-6 flex items-start justify-between">
        <div>
          <div className="flex items-center space-x-2">
            <Radio className="w-6 h-6 text-amber-400 animate-pulse" />
            <h2 className="text-xl font-bold text-white">Soroban Event Log Stream</h2>
          </div>
          <p className="text-xs text-slate-400 mt-1">
            Real-time Soroban contract event topics and data payloads emitted during transaction executions for indexer sync.
          </p>
        </div>

        <div className="relative w-72">
          <Search className="w-4 h-4 text-slate-400 absolute left-3 top-2.5" />
          <input
            type="text"
            placeholder="Filter topics or payloads..."
            value={searchTopic}
            onChange={(e) => setSearchTopic(e.target.value)}
            className="w-full bg-slate-950 border border-slate-800 rounded-xl pl-9 pr-3 py-1.5 text-xs text-white outline-none"
          />
        </div>
      </div>

      <div className="space-y-3">
        {filteredEvents.map((evt) => (
          <div key={evt.id} className="bg-slate-900/80 border border-slate-800 rounded-2xl p-4 font-mono text-xs space-y-2">
            <div className="flex items-center justify-between text-slate-400 border-b border-slate-800/80 pb-2">
              <div className="flex items-center space-x-2">
                <span className="w-2 h-2 rounded-full bg-emerald-400"></span>
                <span className="font-bold text-blue-400">{evt.topic}</span>
              </div>
              <div className="flex items-center space-x-4 text-[11px] text-slate-500">
                <span>Ledger #{evt.ledgerSequence}</span>
                <span>{new Date(evt.timestamp).toLocaleTimeString()}</span>
              </div>
            </div>

            <div className="p-3 bg-slate-950 rounded-xl text-emerald-400 overflow-x-auto">
              <pre>{JSON.stringify(evt.data, null, 2)}</pre>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
