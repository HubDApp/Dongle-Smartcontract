export type UserRole = 'admin' | 'owner' | 'reviewer';

export interface UserAccount {
  address: string;
  name: string;
  role: UserRole;
  avatarUrl?: string;
}

export type ProjectStatus = 'active' | 'archived' | 'claimable';
export type VerificationBadge = 'none' | 'verified' | 'gold_partner' | 'audited_security';

export interface Maintainer {
  name: string;
  email: string;
  url?: string;
  role: string;
}

export interface SecurityContact {
  contact: string;
  proofCid: string;
  verified: boolean;
}

export interface Project {
  id: number;
  slug: string;
  name: string;
  description: string;
  website: string;
  repository?: string;
  documentation?: string;
  category: string;
  tags: string[];
  owner: string;
  maintainers: Maintainer[];
  metadataCid: string;
  logoCid?: string;
  bannerCid?: string;
  verified: boolean;
  verificationBadge: VerificationBadge;
  verificationExpiresAt?: number; // Unix timestamp
  featured: boolean;
  featuredRank?: number;
  ratingAverage: number;
  ratingCount: number;
  reviewCount: number;
  status: ProjectStatus;
  createdAt: number;
  ttlLedgers: number;
  securityContact?: SecurityContact;
  linkedProjectIds?: number[];
  dependencies?: { name: string; version: string; type: string }[];
}

export interface Review {
  id: number;
  projectId: number;
  reviewer: string;
  rating: number; // 1 to 5
  title: string;
  comment: string;
  cid?: string;
  isHidden: boolean;
  isReported: boolean;
  createdAt: number;
}

export interface Collection {
  id: number;
  name: string;
  description: string;
  category: string;
  projectIds: number[];
  creator: string;
  isFeatured: boolean;
  createdAt: number;
}

export type VerificationStatus = 'pending' | 'approved' | 'rejected';

export interface VerificationRequest {
  id: number;
  projectId: number;
  requester: string;
  badgeLevel: VerificationBadge;
  status: VerificationStatus;
  notes?: string;
  requestedAt: number;
  reviewedAt?: number;
}

export type DisputeStatus = 'open' | 'resolved' | 'dismissed';

export interface DuplicateDispute {
  id: number;
  projectId: number;
  duplicateOfProjectId: number;
  reporter: string;
  reason: string;
  status: DisputeStatus;
  resolutionNotes?: string;
  createdAt: number;
}

export interface ContractEvent {
  id: string;
  topic: string;
  data: Record<string, any>;
  timestamp: number;
  ledgerSequence: number;
  contractId: string;
}

export interface AdminActionLog {
  id: number;
  admin: string;
  action: string;
  details: string;
  timestamp: number;
}

export interface FeeConfig {
  feeToken: string; // Native XLM or SAC token contract address
  registrationFee: number; // in XLM / token units
  verificationFee: number;
  featureFee: number;
  reviewFee: number;
  isFeeEnabled: boolean;
}

export interface ContractStats {
  totalProjects: number;
  totalVerified: number;
  totalReviews: number;
  totalCollections: number;
  totalAdmins: number;
  totalDisputes: number;
  contractTtlLedgers: number;
  currentLedger: number;
}
