import React, { useState } from 'react';
import { Layers, Plus, ShieldCheck, ExternalLink, Sparkles } from 'lucide-react';
import { Collection, Project, UserAccount } from '../types';
import { contractEngine } from '../services/contractEngine';

interface CollectionsViewProps {
  currentUser: UserAccount;
  onSelectProject: (p: Project) => void;
}

export const CollectionsView: React.FC<CollectionsViewProps> = ({ currentUser, onSelectProject }) => {
  const [collections, setCollections] = useState<Collection[]>(contractEngine.getCollections());
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [category, setCategory] = useState('DeFi');
  const [selectedProjectIds, setSelectedProjectIds] = useState<number[]>([]);

  const allProjects = contractEngine.getProjects();

  const handleCreateCollection = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !description.trim()) return;

    try {
      contractEngine.createCollection(
        currentUser.address,
        name.trim(),
        description.trim(),
        category,
        selectedProjectIds
      );
      setCollections(contractEngine.getCollections());
      setShowCreateModal(false);
      setName('');
      setDescription('');
      setSelectedProjectIds([]);
    } catch (e: any) {
      alert(e.message);
    }
  };

  const toggleProjectSelection = (id: number) => {
    if (selectedProjectIds.includes(id)) {
      setSelectedProjectIds(selectedProjectIds.filter((pId) => pId !== id));
    } else {
      setSelectedProjectIds([...selectedProjectIds, id]);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-bold text-white">Curated Collections</h2>
          <p className="text-xs text-slate-400">Admin and community curated project bundles on Stellar Soroban.</p>
        </div>

        <button
          onClick={() => setShowCreateModal(true)}
          className="px-4 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold text-xs transition flex items-center space-x-1.5 shadow-lg shadow-blue-500/20"
        >
          <Plus className="w-4 h-4" />
          <span>Create Collection</span>
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {collections.map((col) => {
          const colProjects = allProjects.filter((p) => col.projectIds.includes(p.id));
          return (
            <div key={col.id} className="bg-slate-900/80 border border-slate-800 rounded-2xl p-6 space-y-4">
              <div className="flex items-start justify-between">
                <div>
                  <span className="text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-blue-500/10 text-blue-400 border border-blue-500/20">
                    {col.category}
                  </span>
                  <h3 className="font-bold text-lg text-white mt-1">{col.name}</h3>
                  <p className="text-xs text-slate-300 mt-1">{col.description}</p>
                </div>
                {col.isFeatured && (
                  <span className="bg-amber-500/10 text-amber-400 text-xs px-2.5 py-1 rounded-full border border-amber-500/30 flex items-center space-x-1">
                    <Sparkles className="w-3 h-3" />
                    <span>Featured</span>
                  </span>
                )}
              </div>

              {/* Collection Projects List */}
              <div className="space-y-2 pt-2 border-t border-slate-800">
                <span className="text-[11px] font-mono text-slate-500 uppercase">Projects in Collection ({colProjects.length}):</span>
                {colProjects.map((proj) => (
                  <div
                    key={proj.id}
                    onClick={() => onSelectProject(proj)}
                    className="p-3 bg-slate-950 hover:bg-slate-800/80 border border-slate-800/80 rounded-xl flex items-center justify-between cursor-pointer transition"
                  >
                    <div className="flex items-center space-x-2">
                      <div className="w-7 h-7 rounded-lg bg-slate-800 flex items-center justify-center text-xs font-bold text-blue-400">
                        {proj.name.charAt(0)}
                      </div>
                      <div>
                        <div className="font-medium text-white text-xs">{proj.name}</div>
                        <div className="text-[10px] font-mono text-slate-500">/{proj.slug}</div>
                      </div>
                    </div>
                    {proj.verified && <ShieldCheck className="w-4 h-4 text-emerald-400" />}
                  </div>
                ))}
              </div>
            </div>
          );
        })}
      </div>

      {/* Create Collection Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 z-50 bg-slate-950/80 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-slate-900 border border-slate-800 rounded-2xl w-full max-w-lg p-6 space-y-4">
            <h3 className="font-bold text-white text-lg">Create Project Collection</h3>

            <form onSubmit={handleCreateCollection} className="space-y-4 text-xs">
              <div>
                <label className="block text-slate-300 mb-1 font-semibold">Collection Name</label>
                <input
                  type="text"
                  placeholder="e.g. Infrastructure Superstars"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-white outline-none"
                  required
                />
              </div>

              <div>
                <label className="block text-slate-300 mb-1 font-semibold">Description</label>
                <textarea
                  rows={2}
                  placeholder="Brief summary of this curated collection..."
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-white outline-none"
                  required
                />
              </div>

              <div>
                <label className="block text-slate-300 mb-1 font-semibold">Select Projects to Include</label>
                <div className="max-h-48 overflow-y-auto space-y-1.5 p-2 bg-slate-950 border border-slate-800 rounded-xl">
                  {allProjects.map((p) => (
                    <label key={p.id} className="flex items-center space-x-2 text-slate-300 hover:bg-slate-900 p-1.5 rounded cursor-pointer">
                      <input
                        type="checkbox"
                        checked={selectedProjectIds.includes(p.id)}
                        onChange={() => toggleProjectSelection(p.id)}
                        className="rounded bg-slate-800 border-slate-700 text-blue-600 focus:ring-0"
                      />
                      <span>{p.name} (/{p.slug})</span>
                    </label>
                  ))}
                </div>
              </div>

              <div className="pt-2 flex justify-end space-x-2">
                <button
                  type="button"
                  onClick={() => setShowCreateModal(false)}
                  className="px-4 py-2 rounded-xl text-slate-400 hover:text-white"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="px-4 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold"
                >
                  Create
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
