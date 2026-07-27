import React, { useState } from 'react';
import { X, Plus, AlertCircle, Coins, ShieldCheck, CheckCircle2 } from 'lucide-react';
import { contractEngine } from '../services/contractEngine';
import { UserAccount, Project } from '../types';

interface RegisterProjectModalProps {
  isOpen: boolean;
  onClose: () => void;
  currentUser: UserAccount;
  onSuccess: (newProject: Project) => void;
}

export const RegisterProjectModal: React.FC<RegisterProjectModalProps> = ({
  isOpen,
  onClose,
  currentUser,
  onSuccess,
}) => {
  const feeConfig = contractEngine.getFeeConfig();
  const [name, setName] = useState('');
  const [slug, setSlug] = useState('');
  const [description, setDescription] = useState('');
  const [website, setWebsite] = useState('');
  const [repository, setRepository] = useState('');
  const [category, setCategory] = useState('DeFi');
  const [tagsInput, setTagsInput] = useState('stellar, soroban');
  const [metadataCid, setMetadataCid] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isOpen) return null;

  const handleNameChange = (val: string) => {
    setName(val);
    if (!slug) {
      setSlug(val.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, ''));
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!name.trim() || !slug.trim() || !description.trim() || !website.trim()) {
      setError('Please fill in all required fields (Name, Slug, Description, Website).');
      return;
    }

    try {
      setIsSubmitting(true);
      const tags = tagsInput
        .split(',')
        .map((t) => t.trim())
        .filter((t) => t.length > 0);

      const generatedCid = metadataCid.trim() || `QmMetadata${Date.now()}${Math.floor(Math.random() * 1000)}`;

      const newProj = contractEngine.registerProject(currentUser.address, {
        name: name.trim(),
        slug: slug.trim(),
        description: description.trim(),
        website: website.trim(),
        repository: repository.trim() || undefined,
        category,
        tags,
        metadataCid: generatedCid,
      });

      setIsSubmitting(false);
      onSuccess(newProj);
      onClose();
    } catch (err: any) {
      setIsSubmitting(false);
      setError(err.message || 'Failed to register project on smart contract.');
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-slate-950/80 backdrop-blur-sm flex items-center justify-center p-4 overflow-y-auto">
      <div className="bg-slate-900 border border-slate-800 rounded-2xl w-full max-w-2xl overflow-hidden shadow-2xl my-8">
        {/* Header */}
        <div className="px-6 py-4 border-b border-slate-800 flex items-center justify-between bg-slate-900/50">
          <div className="flex items-center space-x-2">
            <div className="p-2 rounded-xl bg-blue-500/10 text-blue-400 border border-blue-500/20">
              <Plus className="w-5 h-5" />
            </div>
            <div>
              <h2 className="font-bold text-lg text-white">Register On-Chain Project</h2>
              <p className="text-xs text-slate-400">Invoke Soroban contract method: <span className="font-mono text-blue-400">register_project()</span></p>
            </div>
          </div>
          <button onClick={onClose} className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Fee Banner */}
        <div className="px-6 py-3 bg-blue-950/30 border-b border-blue-900/40 flex items-center justify-between text-xs text-blue-300">
          <div className="flex items-center space-x-2">
            <Coins className="w-4 h-4 text-amber-400" />
            <span>
              Registration Fee: <strong className="text-white font-mono">{feeConfig.registrationFee} XLM</strong>
            </span>
          </div>
          <span className="text-slate-400 text-[11px]">Owner address: <span className="font-mono text-slate-300">{currentUser.address.slice(0, 10)}...</span></span>
        </div>

        {/* Form Body */}
        <form onSubmit={handleSubmit} className="p-6 space-y-4 text-sm">
          {error && (
            <div className="p-3 rounded-xl bg-red-500/10 border border-red-500/30 text-red-400 text-xs flex items-center space-x-2">
              <AlertCircle className="w-4 h-4 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">Project Name *</label>
              <input
                type="text"
                placeholder="e.g. Soroban Swap AMM"
                value={name}
                onChange={(e) => handleNameChange(e.target.value)}
                className="w-full bg-slate-950 border border-slate-800 focus:border-blue-500 rounded-xl px-3 py-2 text-white placeholder-slate-600 outline-none"
                required
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">URL Slug *</label>
              <input
                type="text"
                placeholder="soroban-swap-amm"
                value={slug}
                onChange={(e) => setSlug(e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, ''))}
                className="w-full bg-slate-950 border border-slate-800 focus:border-blue-500 rounded-xl px-3 py-2 font-mono text-xs text-blue-400 placeholder-slate-600 outline-none"
                required
              />
            </div>
          </div>

          <div>
            <label className="block text-xs font-semibold text-slate-300 mb-1">Description *</label>
            <textarea
              rows={3}
              placeholder="Provide a concise description of your project and its protocol features..."
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className="w-full bg-slate-950 border border-slate-800 focus:border-blue-500 rounded-xl px-3 py-2 text-white placeholder-slate-600 outline-none"
              required
            />
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">Website URL *</label>
              <input
                type="url"
                placeholder="https://myproject.org"
                value={website}
                onChange={(e) => setWebsite(e.target.value)}
                className="w-full bg-slate-950 border border-slate-800 focus:border-blue-500 rounded-xl px-3 py-2 text-white placeholder-slate-600 outline-none"
                required
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">GitHub / Code Repo (Optional)</label>
              <input
                type="url"
                placeholder="https://github.com/org/repo"
                value={repository}
                onChange={(e) => setRepository(e.target.value)}
                className="w-full bg-slate-950 border border-slate-800 focus:border-blue-500 rounded-xl px-3 py-2 text-white placeholder-slate-600 outline-none"
              />
            </div>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">Category</label>
              <select
                value={category}
                onChange={(e) => setCategory(e.target.value)}
                className="w-full bg-slate-950 border border-slate-800 focus:border-blue-500 rounded-xl px-3 py-2 text-white outline-none"
              >
                <option value="DeFi">DeFi</option>
                <option value="Infrastructure">Infrastructure</option>
                <option value="Governance">Governance</option>
                <option value="Analytics">Analytics</option>
                <option value="Bridges">Bridges</option>
                <option value="Tooling">Tooling</option>
                <option value="Gaming & NFTs">Gaming & NFTs</option>
              </select>
            </div>

            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">Tags (comma separated)</label>
              <input
                type="text"
                placeholder="stellar, soroban, dex"
                value={tagsInput}
                onChange={(e) => setTagsInput(e.target.value)}
                className="w-full bg-slate-950 border border-slate-800 focus:border-blue-500 rounded-xl px-3 py-2 text-white placeholder-slate-600 outline-none"
              />
            </div>
          </div>

          <div>
            <label className="block text-xs font-semibold text-slate-300 mb-1">
              Metadata IPFS CID (Auto-generated if left blank)
            </label>
            <input
              type="text"
              placeholder="QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"
              value={metadataCid}
              onChange={(e) => setMetadataCid(e.target.value)}
              className="w-full bg-slate-950 border border-slate-800 focus:border-blue-500 rounded-xl px-3 py-2 font-mono text-xs text-slate-300 placeholder-slate-600 outline-none"
            />
          </div>

          {/* Footer Actions */}
          <div className="pt-4 border-t border-slate-800 flex items-center justify-end space-x-3">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-xl text-slate-400 hover:text-white hover:bg-slate-800 transition"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="px-5 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold transition shadow-lg shadow-blue-500/20 flex items-center space-x-2"
            >
              {isSubmitting ? (
                <span>Registering on WASM...</span>
              ) : (
                <>
                  <CheckCircle2 className="w-4 h-4" />
                  <span>Submit & Pay {feeConfig.registrationFee} XLM</span>
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
