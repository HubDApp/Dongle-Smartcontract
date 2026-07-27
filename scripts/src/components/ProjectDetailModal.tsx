import React, { useState } from 'react';
import {
  X,
  ExternalLink,
  ShieldCheck,
  Star,
  Clock,
  Award,
  Layers,
  CheckCircle2,
  AlertTriangle,
  MessageSquare,
  Plus,
  GitBranch,
  Info,
  Shield,
  Trash2,
  EyeOff,
  RefreshCw,
  Coins,
} from 'lucide-react';
import { Project, Review, UserAccount } from '../types';
import { contractEngine } from '../services/contractEngine';

interface ProjectDetailModalProps {
  project: Project | null;
  onClose: () => void;
  currentUser: UserAccount;
  onUpdate: () => void;
  onOpenReviewModal: (projectId: number) => void;
}

export const ProjectDetailModal: React.FC<ProjectDetailModalProps> = ({
  project,
  onClose,
  currentUser,
  onUpdate,
  onOpenReviewModal,
}) => {
  const [activeTab, setActiveTab] = useState<'overview' | 'reviews' | 'verification' | 'disputes'>('overview');
  const [disputeReason, setDisputeReason] = useState('');
  const [disputeTargetId, setDisputeTargetId] = useState<number | ''>('');
  const [disputeSuccess, setDisputeSuccess] = useState(false);
  const [disputeError, setDisputeError] = useState<string | null>(null);
  const [actionNotice, setActionNotice] = useState<string | null>(null);

  if (!project) return null;

  const reviews = contractEngine.getReviewsForProject(project.id);

  const handleExtendTtl = () => {
    try {
      const newTtl = contractEngine.extendTtl(currentUser.address, project.id, 100000);
      setActionNotice(`Extended storage TTL by 100,000 ledgers. New TTL: ${newTtl.toLocaleString()} ledgers.`);
      onUpdate();
    } catch (e: any) {
      setActionNotice(`Failed: ${e.message}`);
    }
  };

  const handleRequestVerification = () => {
    try {
      contractEngine.requestVerification(currentUser.address, project.id, 'verified', 'Owner requested protocol audit verification.');
      setActionNotice('Verification request submitted on-chain for Admin approval!');
      onUpdate();
    } catch (e: any) {
      setActionNotice(`Failed: ${e.message}`);
    }
  };

  const handleHideReview = (reviewId: number) => {
    try {
      contractEngine.hideReview(currentUser.address, reviewId);
      setActionNotice(`Review #${reviewId} has been hidden on-chain by Admin.`);
      onUpdate();
    } catch (e: any) {
      setActionNotice(`Failed: ${e.message}`);
    }
  };

  const handleOpenDispute = (e: React.FormEvent) => {
    e.preventDefault();
    setDisputeError(null);
    setDisputeSuccess(false);

    if (!disputeTargetId || !disputeReason.trim()) {
      setDisputeError('Please select a target duplicate project and state a valid reason.');
      return;
    }

    try {
      contractEngine.openDuplicateDispute(currentUser.address, project.id, Number(disputeTargetId), disputeReason.trim());
      setDisputeSuccess(true);
      setDisputeReason('');
      setDisputeTargetId('');
      onUpdate();
    } catch (err: any) {
      setDisputeError(err.message || 'Failed to open duplicate dispute.');
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-slate-950/80 backdrop-blur-md flex items-center justify-center p-4 overflow-y-auto">
      <div className="bg-slate-900 border border-slate-800 rounded-3xl w-full max-w-4xl overflow-hidden shadow-2xl my-8 flex flex-col max-h-[90vh]">
        {/* Header Banner */}
        <div className="relative bg-gradient-to-r from-blue-900/40 via-purple-900/20 to-slate-900 p-6 border-b border-slate-800 flex items-start justify-between">
          <div className="flex items-start space-x-4">
            <div className="w-16 h-16 rounded-2xl bg-slate-800 border-2 border-slate-700 flex items-center justify-center font-bold text-2xl text-blue-400 shrink-0 shadow-lg">
              {project.name.charAt(0)}
            </div>
            <div>
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="text-2xl font-bold text-white">{project.name}</h2>
                {project.verified && (
                  <span className="inline-flex items-center space-x-1 text-xs font-semibold px-2.5 py-1 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/30">
                    <ShieldCheck className="w-3.5 h-3.5" />
                    <span className="capitalize">{project.verificationBadge.replace('_', ' ')}</span>
                  </span>
                )}
                {project.featured && (
                  <span className="text-xs font-semibold px-2.5 py-1 rounded-full bg-amber-500/20 text-amber-300 border border-amber-500/30">
                    Rank #{project.featuredRank || 1} Featured
                  </span>
                )}
              </div>
              <p className="text-xs font-mono text-slate-400 mt-1">
                Slug: <span className="text-blue-400">/{project.slug}</span> | Owner: <span className="text-slate-300">{project.owner.slice(0, 10)}...</span>
              </p>
            </div>
          </div>

          <button onClick={onClose} className="p-2 rounded-xl text-slate-400 hover:text-white hover:bg-slate-800 transition">
            <X className="w-6 h-6" />
          </button>
        </div>

        {/* Action Notice Bar */}
        {actionNotice && (
          <div className="bg-blue-600/20 border-b border-blue-500/30 px-6 py-2.5 text-xs text-blue-300 flex items-center justify-between">
            <span>{actionNotice}</span>
            <button onClick={() => setActionNotice(null)} className="text-blue-400 hover:text-white underline">Dismiss</button>
          </div>
        )}

        {/* Modal Navigation Tabs */}
        <div className="flex border-b border-slate-800 bg-slate-950/40 px-6 space-x-2">
          {(['overview', 'reviews', 'verification', 'disputes'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`py-3 px-4 text-xs font-semibold uppercase tracking-wider border-b-2 transition ${
                activeTab === tab
                  ? 'border-blue-500 text-blue-400 bg-blue-500/5'
                  : 'border-transparent text-slate-400 hover:text-slate-200'
              }`}
            >
              {tab === 'reviews' ? `Reviews (${reviews.length})` : tab}
            </button>
          ))}
        </div>

        {/* Modal Body Content */}
        <div className="p-6 overflow-y-auto space-y-6 flex-1 text-sm">
          {activeTab === 'overview' && (
            <div className="space-y-6">
              {/* Description & Metadata */}
              <div className="bg-slate-950/50 p-5 rounded-2xl border border-slate-800 space-y-3">
                <h3 className="font-semibold text-white text-base">Project Description</h3>
                <p className="text-slate-300 leading-relaxed">{project.description}</p>

                <div className="pt-3 flex flex-wrap gap-4 text-xs font-mono text-slate-400">
                  <div>
                    <span className="text-slate-500 block text-[10px] uppercase">Website</span>
                    <a href={project.website} target="_blank" rel="noreferrer" className="text-blue-400 hover:underline inline-flex items-center space-x-1">
                      <span>{project.website}</span>
                      <ExternalLink className="w-3 h-3" />
                    </a>
                  </div>

                  {project.repository && (
                    <div>
                      <span className="text-slate-500 block text-[10px] uppercase">Repository</span>
                      <a href={project.repository} target="_blank" rel="noreferrer" className="text-blue-400 hover:underline inline-flex items-center space-x-1">
                        <span>{project.repository}</span>
                        <ExternalLink className="w-3 h-3" />
                      </a>
                    </div>
                  )}

                  <div>
                    <span className="text-slate-500 block text-[10px] uppercase">Metadata CID</span>
                    <span className="text-slate-300">{project.metadataCid.slice(0, 16)}...</span>
                  </div>
                </div>
              </div>

              {/* Security Contact */}
              {project.securityContact && (
                <div className="bg-purple-950/20 border border-purple-900/40 p-4 rounded-2xl flex items-center justify-between text-xs">
                  <div className="flex items-center space-x-3">
                    <Shield className="w-5 h-5 text-purple-400 shrink-0" />
                    <div>
                      <div className="font-semibold text-purple-300">Verified Security Contact</div>
                      <div className="text-slate-400">{project.securityContact.contact}</div>
                    </div>
                  </div>
                  <span className="font-mono text-slate-400 text-[11px]">Proof CID: {project.securityContact.proofCid.slice(0, 12)}...</span>
                </div>
              )}

              {/* Dependencies & Tech Stack */}
              {project.dependencies && project.dependencies.length > 0 && (
                <div className="bg-slate-950/50 p-5 rounded-2xl border border-slate-800">
                  <h4 className="font-semibold text-white mb-3 text-xs uppercase tracking-wider text-slate-400">On-Chain & SDK Dependencies</h4>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    {project.dependencies.map((dep, idx) => (
                      <div key={idx} className="p-3 bg-slate-900 border border-slate-800 rounded-xl flex items-center justify-between text-xs">
                        <div className="flex items-center space-x-2">
                          <GitBranch className="w-4 h-4 text-blue-400" />
                          <span className="font-medium text-slate-200">{dep.name}</span>
                        </div>
                        <span className="font-mono text-slate-400 bg-slate-800 px-2 py-0.5 rounded text-[11px]">{dep.version}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Storage & Actions Bar */}
              <div className="bg-slate-950/50 p-5 rounded-2xl border border-slate-800 flex flex-wrap items-center justify-between gap-4">
                <div>
                  <div className="flex items-center space-x-2 text-xs text-slate-300">
                    <Clock className="w-4 h-4 text-blue-400" />
                    <span>Persistence Ledger TTL: <strong className="font-mono text-white">{project.ttlLedgers.toLocaleString()} ledgers</strong></span>
                  </div>
                  <div className="text-[11px] text-slate-500 mt-1">Automatic Soroban bumpInstanceFootprint active.</div>
                </div>

                <div className="flex flex-wrap items-center gap-2">
                  <button
                    onClick={handleExtendTtl}
                    className="px-3 py-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-200 border border-slate-700 transition flex items-center space-x-1.5"
                  >
                    <RefreshCw className="w-3.5 h-3.5 text-blue-400" />
                    <span>Extend TTL (+100k)</span>
                  </button>

                  <button
                    onClick={() => onOpenReviewModal(project.id)}
                    className="px-4 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold transition shadow-lg shadow-blue-500/20 flex items-center space-x-1.5"
                  >
                    <MessageSquare className="w-3.5 h-3.5" />
                    <span>Write Review</span>
                  </button>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'reviews' && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <h3 className="font-bold text-white text-base">Community Reviews</h3>
                <button
                  onClick={() => onOpenReviewModal(project.id)}
                  className="px-3 py-1.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold transition flex items-center space-x-1.5"
                >
                  <Plus className="w-3.5 h-3.5" />
                  <span>Submit Review</span>
                </button>
              </div>

              {reviews.length === 0 ? (
                <div className="text-center py-12 bg-slate-950/40 rounded-2xl border border-slate-800 text-slate-400">
                  <MessageSquare className="w-8 h-8 mx-auto mb-2 text-slate-600" />
                  <p className="text-sm">No community reviews submitted yet.</p>
                  <p className="text-xs text-slate-500 mt-1">Be the first on-chain reviewer for {project.name}.</p>
                </div>
              ) : (
                <div className="space-y-3">
                  {reviews.map((rev) => (
                    <div key={rev.id} className="p-4 bg-slate-950/60 border border-slate-800 rounded-2xl space-y-2">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center space-x-2">
                          <div className="flex items-center space-x-0.5 text-amber-400">
                            {[...Array(5)].map((_, i) => (
                              <Star
                                key={i}
                                className={`w-3.5 h-3.5 ${
                                  i < rev.rating ? 'fill-amber-400 text-amber-400' : 'text-slate-700'
                                }`}
                              />
                            ))}
                          </div>
                          <span className="font-bold text-white text-xs">{rev.title}</span>
                        </div>

                        {currentUser.role === 'admin' && (
                          <button
                            onClick={() => handleHideReview(rev.id)}
                            className="p-1 text-slate-500 hover:text-red-400 transition"
                            title="Admin: Hide Review"
                          >
                            <EyeOff className="w-4 h-4" />
                          </button>
                        )}
                      </div>

                      <p className="text-xs text-slate-300 leading-relaxed">{rev.comment}</p>

                      <div className="flex items-center justify-between pt-2 text-[10px] font-mono text-slate-500">
                        <span>Reviewer: {rev.reviewer.slice(0, 12)}...</span>
                        <span>{new Date(rev.createdAt).toLocaleDateString()}</span>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {activeTab === 'verification' && (
            <div className="space-y-4">
              <div className="p-5 bg-slate-950/60 border border-slate-800 rounded-2xl space-y-3">
                <h3 className="font-bold text-white text-base flex items-center space-x-2">
                  <ShieldCheck className="w-5 h-5 text-emerald-400" />
                  <span>On-Chain Verification Status</span>
                </h3>

                <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 pt-2">
                  <div className="p-3 bg-slate-900 border border-slate-800 rounded-xl">
                    <span className="text-[10px] text-slate-500 uppercase block font-mono">Current Status</span>
                    <span className="font-semibold text-white capitalize">{project.verified ? project.verificationBadge.replace('_', ' ') : 'Unverified'}</span>
                  </div>

                  <div className="p-3 bg-slate-900 border border-slate-800 rounded-xl">
                    <span className="text-[10px] text-slate-500 uppercase block font-mono">Badge Level</span>
                    <span className="font-semibold text-emerald-400">{project.verificationBadge}</span>
                  </div>

                  <div className="p-3 bg-slate-900 border border-slate-800 rounded-xl">
                    <span className="text-[10px] text-slate-500 uppercase block font-mono">Expiration</span>
                    <span className="font-semibold text-slate-300">
                      {project.verificationExpiresAt ? new Date(project.verificationExpiresAt).toLocaleDateString() : 'N/A'}
                    </span>
                  </div>
                </div>

                <div className="pt-3 border-t border-slate-800 flex items-center justify-between">
                  <p className="text-xs text-slate-400">
                    Verification badges are managed by contract admins after verifying security contacts & GitHub repo proofs.
                  </p>
                  <button
                    onClick={handleRequestVerification}
                    className="px-4 py-2 rounded-xl bg-purple-600 hover:bg-purple-500 text-white text-xs font-semibold transition"
                  >
                    Request Protocol Verification
                  </button>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'disputes' && (
            <div className="space-y-4">
              <div className="p-5 bg-slate-950/60 border border-slate-800 rounded-2xl space-y-4">
                <h3 className="font-bold text-white text-base flex items-center space-x-2">
                  <AlertTriangle className="w-5 h-5 text-amber-400" />
                  <span>Open Duplicate Dispute</span>
                </h3>
                <p className="text-xs text-slate-400">
                  If another registered project is an unauthorized copy or abandoned clone of {project.name}, report it for admin resolution.
                </p>

                {disputeSuccess && (
                  <div className="p-3 bg-emerald-500/10 border border-emerald-500/30 rounded-xl text-emerald-400 text-xs">
                    Dispute reported successfully! Admin will review in moderation queue.
                  </div>
                )}

                {disputeError && (
                  <div className="p-3 bg-red-500/10 border border-red-500/30 rounded-xl text-red-400 text-xs">
                    {disputeError}
                  </div>
                )}

                <form onSubmit={handleOpenDispute} className="space-y-3">
                  <div>
                    <label className="block text-xs font-medium text-slate-300 mb-1">Select Alleged Duplicate Project ID</label>
                    <select
                      value={disputeTargetId}
                      onChange={(e) => setDisputeTargetId(Number(e.target.value))}
                      className="w-full bg-slate-900 border border-slate-800 rounded-xl px-3 py-2 text-xs text-white outline-none"
                    >
                      <option value="">Select a project...</option>
                      {contractEngine
                        .getProjects()
                        .filter((p) => p.id !== project.id)
                        .map((p) => (
                          <option key={p.id} value={p.id}>
                            #{p.id} - {p.name} (/{p.slug})
                          </option>
                        ))}
                    </select>
                  </div>

                  <div>
                    <label className="block text-xs font-medium text-slate-300 mb-1">Reason & Evidence Details</label>
                    <textarea
                      rows={3}
                      placeholder="Explain why this project is a duplicate or copyright infringement..."
                      value={disputeReason}
                      onChange={(e) => setDisputeReason(e.target.value)}
                      className="w-full bg-slate-900 border border-slate-800 rounded-xl px-3 py-2 text-xs text-white placeholder-slate-600 outline-none"
                    />
                  </div>

                  <button
                    type="submit"
                    className="px-4 py-2 rounded-xl bg-amber-600 hover:bg-amber-500 text-white text-xs font-semibold transition"
                  >
                    Submit On-Chain Dispute
                  </button>
                </form>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
