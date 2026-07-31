import React, { useState } from 'react';
import { X, Star, AlertCircle, CheckCircle2 } from 'lucide-react';
import { contractEngine } from '../services/contractEngine';
import { UserAccount } from '../types';

interface SubmitReviewModalProps {
  isOpen: boolean;
  projectId: number | null;
  onClose: () => void;
  currentUser: UserAccount;
  onSuccess: () => void;
}

export const SubmitReviewModal: React.FC<SubmitReviewModalProps> = ({
  isOpen,
  projectId,
  onClose,
  currentUser,
  onSuccess,
}) => {
  const [rating, setRating] = useState<number>(5);
  const [title, setTitle] = useState('');
  const [comment, setComment] = useState('');
  const [error, setError] = useState<string | null>(null);

  if (!isOpen || !projectId) return null;

  const project = contractEngine.getProjectById(projectId);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!title.trim() || !comment.trim()) {
      setError('Please provide both a title and review content.');
      return;
    }

    try {
      contractEngine.submitReview(
        currentUser.address,
        projectId,
        rating,
        title.trim(),
        comment.trim()
      );
      onSuccess();
      onClose();
    } catch (err: any) {
      setError(err.message || 'Failed to submit review.');
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-slate-950/80 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-slate-800 rounded-2xl w-full max-w-lg overflow-hidden shadow-2xl">
        <div className="px-6 py-4 border-b border-slate-800 flex items-center justify-between">
          <h3 className="font-bold text-white text-base">Submit On-Chain Review</h3>
          <button onClick={onClose} className="p-1 rounded-lg text-slate-400 hover:text-white">
            <X className="w-5 h-5" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-6 space-y-4 text-sm">
          {error && (
            <div className="p-3 rounded-xl bg-red-500/10 border border-red-500/30 text-red-400 text-xs flex items-center space-x-2">
              <AlertCircle className="w-4 h-4 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          <div className="text-xs text-slate-400">
            Reviewing project: <strong className="text-white">{project?.name}</strong>
          </div>

          <div>
            <label className="block text-xs font-semibold text-slate-300 mb-2">Rating (1 to 5 Stars)</label>
            <div className="flex items-center space-x-2">
              {[1, 2, 3, 4, 5].map((star) => (
                <button
                  key={star}
                  type="button"
                  onClick={() => setRating(star)}
                  className="p-1 hover:scale-110 transition-transform focus:outline-none"
                >
                  <Star
                    className={`w-7 h-7 ${
                      star <= rating ? 'fill-amber-400 text-amber-400' : 'text-slate-700'
                    }`}
                  />
                </button>
              ))}
              <span className="text-sm font-bold text-amber-400 ml-2">{rating} / 5 Stars</span>
            </div>
          </div>

          <div>
            <label className="block text-xs font-semibold text-slate-300 mb-1">Review Headline *</label>
            <input
              type="text"
              placeholder="e.g. Excellent Soroban contract integration"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-white placeholder-slate-600 outline-none"
              required
            />
          </div>

          <div>
            <label className="block text-xs font-semibold text-slate-300 mb-1">Detailed Review *</label>
            <textarea
              rows={4}
              placeholder="Share your experience with contract reliability, gas usage, and docs..."
              value={comment}
              onChange={(e) => setComment(e.target.value)}
              className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-white placeholder-slate-600 outline-none"
              required
            />
          </div>

          <div className="pt-3 border-t border-slate-800 flex items-center justify-end space-x-3">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-xl text-slate-400 hover:text-white"
            >
              Cancel
            </button>
            <button
              type="submit"
              className="px-5 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold transition flex items-center space-x-1.5"
            >
              <CheckCircle2 className="w-4 h-4" />
              <span>Submit Review</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
