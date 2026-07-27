import React from 'react';
import { ShieldCheck, Star, ExternalLink, Tag, Clock, Layers, ShieldAlert, Award } from 'lucide-react';
import { Project, VerificationBadge } from '../types';

interface ProjectCardProps {
  project: Project;
  onSelect: (project: Project) => void;
}

export const ProjectCard: React.FC<ProjectCardProps> = ({ project, onSelect }) => {
  const getBadgeStyle = (badge: VerificationBadge) => {
    switch (badge) {
      case 'gold_partner':
        return {
          label: 'Gold Partner',
          bg: 'bg-amber-500/10 border-amber-500/30 text-amber-400',
          icon: Award,
        };
      case 'audited_security':
        return {
          label: 'Security Audited',
          bg: 'bg-purple-500/10 border-purple-500/30 text-purple-400',
          icon: ShieldCheck,
        };
      case 'verified':
        return {
          label: 'Verified',
          bg: 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400',
          icon: ShieldCheck,
        };
      default:
        return null;
    }
  };

  const badgeInfo = getBadgeStyle(project.verificationBadge);
  const BadgeIcon = badgeInfo?.icon;

  return (
    <div
      onClick={() => onSelect(project)}
      className="group relative bg-slate-900/60 hover:bg-slate-800/80 border border-slate-800 hover:border-blue-500/40 rounded-2xl p-5 transition-all duration-200 cursor-pointer flex flex-col justify-between shadow-lg hover:shadow-blue-500/5"
    >
      <div>
        {/* Top Header Row */}
        <div className="flex items-start justify-between gap-3 mb-3">
          <div className="flex items-center space-x-3">
            <div className="w-12 h-12 rounded-xl bg-slate-800 border border-slate-700 flex items-center justify-center font-bold text-lg text-blue-400 shrink-0 group-hover:scale-105 transition-transform">
              {project.name.charAt(0)}
            </div>
            <div>
              <div className="flex items-center space-x-2">
                <h3 className="font-bold text-white text-base group-hover:text-blue-400 transition-colors line-clamp-1">
                  {project.name}
                </h3>
                {project.featured && (
                  <span className="bg-amber-500/20 text-amber-300 text-[10px] font-semibold px-2 py-0.5 rounded-full border border-amber-500/30">
                    Featured
                  </span>
                )}
              </div>
              <p className="text-xs font-mono text-slate-400">/{project.slug}</p>
            </div>
          </div>

          {/* Category Tag */}
          <span className="text-xs font-medium px-2.5 py-1 rounded-lg bg-slate-800 text-slate-300 border border-slate-700/60 shrink-0">
            {project.category}
          </span>
        </div>

        {/* Description */}
        <p className="text-xs text-slate-300 line-clamp-2 mb-4 leading-relaxed">
          {project.description}
        </p>

        {/* Verification & Ratings Line */}
        <div className="flex flex-wrap items-center gap-2 mb-4">
          {badgeInfo && BadgeIcon && (
            <span className={`inline-flex items-center space-x-1 text-xs font-medium px-2.5 py-1 rounded-lg border ${badgeInfo.bg}`}>
              <BadgeIcon className="w-3.5 h-3.5" />
              <span>{badgeInfo.label}</span>
            </span>
          )}

          {/* Rating */}
          <div className="inline-flex items-center space-x-1 text-xs font-semibold px-2.5 py-1 rounded-lg bg-slate-800/80 border border-slate-700 text-amber-400">
            <Star className="w-3.5 h-3.5 fill-amber-400 text-amber-400" />
            <span>{project.ratingAverage > 0 ? project.ratingAverage.toFixed(1) : 'Unrated'}</span>
            <span className="text-slate-500 font-normal">({project.reviewCount})</span>
          </div>

          {/* Status */}
          {project.status === 'archived' && (
            <span className="text-xs px-2 py-0.5 rounded bg-red-500/10 text-red-400 border border-red-500/20">
              Archived
            </span>
          )}
          {project.status === 'claimable' && (
            <span className="text-xs px-2 py-0.5 rounded bg-yellow-500/10 text-yellow-400 border border-yellow-500/20">
              Claimable
            </span>
          )}
        </div>

        {/* Tags */}
        <div className="flex flex-wrap gap-1.5 mb-4">
          {project.tags.map((tag) => (
            <span key={tag} className="text-[11px] font-mono px-2 py-0.5 rounded bg-slate-800/50 text-slate-400 border border-slate-800">
              #{tag}
            </span>
          ))}
        </div>
      </div>

      {/* Footer Info */}
      <div className="pt-3 border-t border-slate-800/80 flex items-center justify-between text-xs text-slate-400 font-mono">
        <div className="flex items-center space-x-1 text-slate-500">
          <Clock className="w-3.5 h-3.5" />
          <span>TTL: {(project.ttlLedgers / 1000).toFixed(0)}k ledgers</span>
        </div>
        <div className="flex items-center space-x-2">
          {project.website && (
            <a
              href={project.website}
              target="_blank"
              rel="noreferrer"
              onClick={(e) => e.stopPropagation()}
              className="text-slate-400 hover:text-blue-400 transition"
            >
              <ExternalLink className="w-3.5 h-3.5" />
            </a>
          )}
          <span className="text-blue-400 font-sans font-semibold group-hover:translate-x-0.5 transition-transform">
            View Details &rarr;
          </span>
        </div>
      </div>
    </div>
  );
};
