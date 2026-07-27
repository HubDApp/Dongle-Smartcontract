import {
  Project,
  Review,
  Collection,
  VerificationRequest,
  DuplicateDispute,
  ContractEvent,
  AdminActionLog,
  FeeConfig,
  ContractStats,
  VerificationBadge,
} from '../types';
import {
  INITIAL_PROJECTS,
  INITIAL_REVIEWS,
  INITIAL_COLLECTIONS,
  INITIAL_VERIFICATION_REQUESTS,
  INITIAL_DISPUTES,
  INITIAL_FEE_CONFIG,
  INITIAL_ADMIN_LOGS,
  INITIAL_EVENTS,
  TESTNET_CONTRACT_ID,
} from '../data/mockContractData';

const STORAGE_KEYS = {
  PROJECTS: 'dongle_projects_v1',
  REVIEWS: 'dongle_reviews_v1',
  COLLECTIONS: 'dongle_collections_v1',
  VERIFICATIONS: 'dongle_verifications_v1',
  DISPUTES: 'dongle_disputes_v1',
  FEE_CONFIG: 'dongle_fee_config_v1',
  ADMIN_LOGS: 'dongle_admin_logs_v1',
  EVENTS: 'dongle_events_v1',
  ADMIN_LIST: 'dongle_admin_list_v1',
};

class ContractEngineService {
  private projects: Project[];
  private reviews: Review[];
  private collections: Collection[];
  private verifications: VerificationRequest[];
  private disputes: DuplicateDispute[];
  private feeConfig: FeeConfig;
  private adminLogs: AdminActionLog[];
  private events: ContractEvent[];
  private adminList: string[];
  private ledgerSeq: number = 5520100;

  constructor() {
    this.projects = this.load(STORAGE_KEYS.PROJECTS, INITIAL_PROJECTS);
    this.reviews = this.load(STORAGE_KEYS.REVIEWS, INITIAL_REVIEWS);
    this.collections = this.load(STORAGE_KEYS.COLLECTIONS, INITIAL_COLLECTIONS);
    this.verifications = this.load(STORAGE_KEYS.VERIFICATIONS, INITIAL_VERIFICATION_REQUESTS);
    this.disputes = this.load(STORAGE_KEYS.DISPUTES, INITIAL_DISPUTES);
    this.feeConfig = this.load(STORAGE_KEYS.FEE_CONFIG, INITIAL_FEE_CONFIG);
    this.adminLogs = this.load(STORAGE_KEYS.ADMIN_LOGS, INITIAL_ADMIN_LOGS);
    this.events = this.load(STORAGE_KEYS.EVENTS, INITIAL_EVENTS);
    this.adminList = this.load(STORAGE_KEYS.ADMIN_LIST, [
      'GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N',
    ]);
  }

  private load<T>(key: string, fallback: T): T {
    try {
      const data = localStorage.getItem(key);
      return data ? JSON.parse(data) : fallback;
    } catch {
      return fallback;
    }
  }

  private save(key: string, data: any) {
    try {
      localStorage.setItem(key, JSON.stringify(data));
    } catch (e) {
      console.error('LocalStorage write error:', e);
    }
  }

  private emitEvent(topic: string, data: Record<string, any>) {
    this.ledgerSeq += 1;
    const evt: ContractEvent = {
      id: `evt-${Date.now()}-${Math.floor(Math.random() * 1000)}`,
      topic,
      data,
      timestamp: Date.now(),
      ledgerSequence: this.ledgerSeq,
      contractId: TESTNET_CONTRACT_ID,
    };
    this.events = [evt, ...this.events];
    this.save(STORAGE_KEYS.EVENTS, this.events);
  }

  private logAdminAction(admin: string, action: string, details: string) {
    const log: AdminActionLog = {
      id: this.adminLogs.length + 1,
      admin,
      action,
      details,
      timestamp: Date.now(),
    };
    this.adminLogs = [log, ...this.adminLogs];
    this.save(STORAGE_KEYS.ADMIN_LOGS, this.adminLogs);
  }

  // --- PROJECTS ---
  public getProjects(): Project[] {
    return this.projects;
  }

  public getProjectById(id: number): Project | undefined {
    return this.projects.find((p) => p.id === id);
  }

  public getProjectBySlug(slug: string): Project | undefined {
    return this.projects.find((p) => p.slug === slug);
  }

  public registerProject(
    caller: string,
    data: {
      name: string;
      slug: string;
      description: string;
      website: string;
      category: string;
      tags: string[];
      metadataCid: string;
      repository?: string;
      documentation?: string;
    }
  ): Project {
    const existingSlug = this.projects.find((p) => p.slug.toLowerCase() === data.slug.toLowerCase());
    if (existingSlug) {
      throw new Error(`ErrSlugAlreadyExists: Project slug '${data.slug}' is already taken.`);
    }

    const id = this.projects.length + 1;
    const newProject: Project = {
      id,
      slug: data.slug.toLowerCase(),
      name: data.name,
      description: data.description,
      website: data.website,
      repository: data.repository,
      documentation: data.documentation,
      category: data.category || 'General',
      tags: data.tags || [],
      owner: caller,
      maintainers: [{ name: 'Project Owner', email: 'owner@example.com', role: 'owner' }],
      metadataCid: data.metadataCid || 'QmDefaultMetadataCid0000000000000000000000',
      verified: false,
      verificationBadge: 'none',
      featured: false,
      ratingAverage: 0,
      ratingCount: 0,
      reviewCount: 0,
      status: 'active',
      createdAt: Date.now(),
      ttlLedgers: 100000,
    };

    this.projects = [newProject, ...this.projects];
    this.save(STORAGE_KEYS.PROJECTS, this.projects);

    this.emitEvent('Dongle:register_project', {
      project_id: id,
      slug: newProject.slug,
      owner: caller,
      fee_paid: this.feeConfig.isFeeEnabled ? this.feeConfig.registrationFee : 0,
    });

    return newProject;
  }

  public updateProject(
    caller: string,
    id: number,
    updates: Partial<Pick<Project, 'name' | 'description' | 'website' | 'category' | 'tags' | 'metadataCid' | 'repository' | 'documentation'>>
  ): Project {
    const projIndex = this.projects.findIndex((p) => p.id === id);
    if (projIndex === -1) throw new Error('ErrProjectNotFound');

    const proj = this.projects[projIndex];
    if (proj.owner !== caller && !this.adminList.includes(caller)) {
      throw new Error('ErrUnauthorized: Only project owner or admin can update metadata.');
    }

    const updated = {
      ...proj,
      ...updates,
      tags: updates.tags || proj.tags,
    };

    this.projects[projIndex] = updated;
    this.save(STORAGE_KEYS.PROJECTS, this.projects);

    this.emitEvent('Dongle:update_project', {
      project_id: id,
      updated_by: caller,
      cid: updated.metadataCid,
    });

    return updated;
  }

  public archiveProject(caller: string, id: number): Project {
    const projIndex = this.projects.findIndex((p) => p.id === id);
    if (projIndex === -1) throw new Error('ErrProjectNotFound');

    const proj = this.projects[projIndex];
    if (proj.owner !== caller && !this.adminList.includes(caller)) {
      throw new Error('ErrUnauthorized');
    }

    proj.status = 'archived';
    this.projects[projIndex] = proj;
    this.save(STORAGE_KEYS.PROJECTS, this.projects);

    this.emitEvent('Dongle:archive_project', { project_id: id, archived_by: caller });
    return proj;
  }

  public reactivateProject(caller: string, id: number): Project {
    const projIndex = this.projects.findIndex((p) => p.id === id);
    if (projIndex === -1) throw new Error('ErrProjectNotFound');

    const proj = this.projects[projIndex];
    if (proj.owner !== caller && !this.adminList.includes(caller)) {
      throw new Error('ErrUnauthorized');
    }

    proj.status = 'active';
    this.projects[projIndex] = proj;
    this.save(STORAGE_KEYS.PROJECTS, this.projects);

    this.emitEvent('Dongle:reactivate_project', { project_id: id, reactivated_by: caller });
    return proj;
  }

  // --- REVIEWS & RATINGS ---
  public getReviewsForProject(projectId: number): Review[] {
    return this.reviews.filter((r) => r.projectId === projectId && !r.isHidden);
  }

  public submitReview(
    reviewer: string,
    projectId: number,
    rating: number,
    title: string,
    comment: string,
    cid?: string
  ): Review {
    if (rating < 1 || rating > 5) {
      throw new Error('ErrInvalidRating: Rating must be an integer between 1 and 5.');
    }

    const projIndex = this.projects.findIndex((p) => p.id === projectId);
    if (projIndex === -1) throw new Error('ErrProjectNotFound');

    const proj = this.projects[projIndex];
    if (proj.owner === reviewer) {
      throw new Error('ErrOwnerCannotReview: Project owners cannot submit reviews for their own project.');
    }

    const existingReview = this.reviews.find((r) => r.projectId === projectId && r.reviewer === reviewer);
    if (existingReview) {
      throw new Error('ErrDuplicateReview: You have already submitted a review for this project.');
    }

    const reviewId = this.reviews.length + 1;
    const newReview: Review = {
      id: reviewId,
      projectId,
      reviewer,
      rating,
      title,
      comment,
      cid: cid || `QmReviewCid${reviewId}`,
      isHidden: false,
      isReported: false,
      createdAt: Date.now(),
    };

    this.reviews = [newReview, ...this.reviews];
    this.save(STORAGE_KEYS.REVIEWS, this.reviews);

    // Recalculate project rating
    const projReviews = this.reviews.filter((r) => r.projectId === projectId && !r.isHidden);
    const sum = projReviews.reduce((acc, curr) => acc + curr.rating, 0);
    const newAvg = Number((sum / projReviews.length).toFixed(1));

    proj.ratingAverage = newAvg;
    proj.ratingCount = projReviews.length;
    proj.reviewCount = projReviews.length;
    this.projects[projIndex] = proj;
    this.save(STORAGE_KEYS.PROJECTS, this.projects);

    this.emitEvent('Dongle:submit_review', {
      review_id: reviewId,
      project_id: projectId,
      reviewer,
      rating,
    });

    return newReview;
  }

  public hideReview(admin: string, reviewId: number): void {
    if (!this.adminList.includes(admin)) throw new Error('ErrUnauthorized: Admin role required.');
    const review = this.reviews.find((r) => r.id === reviewId);
    if (!review) throw new Error('ErrReviewNotFound');

    review.isHidden = true;
    this.save(STORAGE_KEYS.REVIEWS, this.reviews);

    this.logAdminAction(admin, 'HIDE_REVIEW', `Hid review #${reviewId} on project #${review.projectId}`);
    this.emitEvent('Dongle:hide_review', { review_id: reviewId, admin });
  }

  // --- VERIFICATION ---
  public requestVerification(caller: string, projectId: number, badgeLevel: VerificationBadge, notes?: string): VerificationRequest {
    const proj = this.projects.find((p) => p.id === projectId);
    if (!proj) throw new Error('ErrProjectNotFound');
    if (proj.owner !== caller && !this.adminList.includes(caller)) throw new Error('ErrUnauthorized');

    const reqId = this.verifications.length + 1;
    const request: VerificationRequest = {
      id: reqId,
      projectId,
      requester: caller,
      badgeLevel,
      status: 'pending',
      notes,
      requestedAt: Date.now(),
    };

    this.verifications = [request, ...this.verifications];
    this.save(STORAGE_KEYS.VERIFICATIONS, this.verifications);

    this.emitEvent('Dongle:request_verification', { request_id: reqId, project_id: projectId, badgeLevel });
    return request;
  }

  public approveVerification(admin: string, requestId: number, durationDays: number = 365): VerificationRequest {
    if (!this.adminList.includes(admin)) throw new Error('ErrUnauthorized: Admin role required');

    const reqIndex = this.verifications.findIndex((v) => v.id === requestId);
    if (reqIndex === -1) throw new Error('ErrRequestNotFound');

    const req = this.verifications[reqIndex];
    req.status = 'approved';
    req.reviewedAt = Date.now();
    this.verifications[reqIndex] = req;
    this.save(STORAGE_KEYS.VERIFICATIONS, this.verifications);

    const projIndex = this.projects.findIndex((p) => p.id === req.projectId);
    if (projIndex !== -1) {
      const proj = this.projects[projIndex];
      proj.verified = true;
      proj.verificationBadge = req.badgeLevel;
      proj.verificationExpiresAt = Date.now() + durationDays * 86400 * 1000;
      this.projects[projIndex] = proj;
      this.save(STORAGE_KEYS.PROJECTS, this.projects);
    }

    this.logAdminAction(admin, 'APPROVE_VERIFICATION', `Approved request #${requestId} for project #${req.projectId}`);
    this.emitEvent('Dongle:approve_verification', {
      request_id: requestId,
      project_id: req.projectId,
      badge: req.badgeLevel,
      expiresAt: req.reviewedAt! + durationDays * 86400 * 1000,
    });

    return req;
  }

  public rejectVerification(admin: string, requestId: number, notes?: string): VerificationRequest {
    if (!this.adminList.includes(admin)) throw new Error('ErrUnauthorized: Admin role required');

    const reqIndex = this.verifications.findIndex((v) => v.id === requestId);
    if (reqIndex === -1) throw new Error('ErrRequestNotFound');

    const req = this.verifications[reqIndex];
    req.status = 'rejected';
    req.notes = notes || req.notes;
    req.reviewedAt = Date.now();
    this.verifications[reqIndex] = req;
    this.save(STORAGE_KEYS.VERIFICATIONS, this.verifications);

    this.logAdminAction(admin, 'REJECT_VERIFICATION', `Rejected verification #${requestId} with notes: ${notes}`);
    return req;
  }

  // --- FEATURED & COLLECTIONS ---
  public setFeatured(admin: string, projectId: number, isFeatured: boolean, rank: number = 1): void {
    if (!this.adminList.includes(admin)) throw new Error('ErrUnauthorized: Admin required');

    const projIndex = this.projects.findIndex((p) => p.id === projectId);
    if (projIndex === -1) throw new Error('ErrProjectNotFound');

    this.projects[projIndex].featured = isFeatured;
    this.projects[projIndex].featuredRank = rank;
    this.save(STORAGE_KEYS.PROJECTS, this.projects);

    this.logAdminAction(admin, 'SET_FEATURED', `Set project #${projectId} featured = ${isFeatured}`);
    this.emitEvent('Dongle:set_featured', { project_id: projectId, featured: isFeatured, rank });
  }

  public getCollections(): Collection[] {
    return this.collections;
  }

  public createCollection(creator: string, name: string, description: string, category: string, projectIds: number[]): Collection {
    const colId = this.collections.length + 1;
    const col: Collection = {
      id: colId,
      name,
      description,
      category,
      projectIds,
      creator,
      isFeatured: false,
      createdAt: Date.now(),
    };

    this.collections = [col, ...this.collections];
    this.save(STORAGE_KEYS.COLLECTIONS, this.collections);

    this.emitEvent('Dongle:create_collection', { collection_id: colId, name, creator });
    return col;
  }

  public addProjectToCollection(caller: string, collectionId: number, projectId: number): Collection {
    const colIndex = this.collections.findIndex((c) => c.id === collectionId);
    if (colIndex === -1) throw new Error('ErrCollectionNotFound');

    const col = this.collections[colIndex];
    if (!col.projectIds.includes(projectId)) {
      col.projectIds.push(projectId);
      this.collections[colIndex] = col;
      this.save(STORAGE_KEYS.COLLECTIONS, this.collections);
    }
    return col;
  }

  // --- DISPUTES ---
  public openDuplicateDispute(reporter: string, projectId: number, duplicateOfProjectId: number, reason: string): DuplicateDispute {
    const dId = this.disputes.length + 1;
    const dispute: DuplicateDispute = {
      id: dId,
      projectId,
      duplicateOfProjectId,
      reporter,
      reason,
      status: 'open',
      createdAt: Date.now(),
    };

    this.disputes = [dispute, ...this.disputes];
    this.save(STORAGE_KEYS.DISPUTES, this.disputes);

    this.emitEvent('Dongle:open_duplicate_dispute', { dispute_id: dId, project_id: projectId, reporter });
    return dispute;
  }

  public resolveDuplicateDispute(admin: string, disputeId: number, action: 'archive_project' | 'dismiss', notes: string): DuplicateDispute {
    if (!this.adminList.includes(admin)) throw new Error('ErrUnauthorized: Admin required');

    const dIndex = this.disputes.findIndex((d) => d.id === disputeId);
    if (dIndex === -1) throw new Error('ErrDisputeNotFound');

    const dispute = this.disputes[dIndex];
    dispute.status = action === 'archive_project' ? 'resolved' : 'dismissed';
    dispute.resolutionNotes = notes;
    this.disputes[dIndex] = dispute;
    this.save(STORAGE_KEYS.DISPUTES, this.disputes);

    if (action === 'archive_project') {
      this.archiveProject(admin, dispute.projectId);
    }

    this.logAdminAction(admin, 'RESOLVE_DISPUTE', `Resolved dispute #${disputeId} with action ${action}`);
    return dispute;
  }

  // --- TTL & PERSISTENCE ---
  public extendTtl(caller: string, projectId: number, additionalLedgers: number = 100000): number {
    const projIndex = this.projects.findIndex((p) => p.id === projectId);
    if (projIndex === -1) throw new Error('ErrProjectNotFound');

    this.projects[projIndex].ttlLedgers += additionalLedgers;
    this.save(STORAGE_KEYS.PROJECTS, this.projects);

    this.emitEvent('Dongle:extend_ttl', { project_id: projectId, extended_by: additionalLedgers });
    return this.projects[projIndex].ttlLedgers;
  }

  // --- ADMIN & FEES ---
  public getFeeConfig(): FeeConfig {
    return this.feeConfig;
  }

  public updateFeeConfig(admin: string, config: Partial<FeeConfig>): FeeConfig {
    if (!this.adminList.includes(admin)) throw new Error('ErrUnauthorized: Admin required');

    this.feeConfig = { ...this.feeConfig, ...config };
    this.save(STORAGE_KEYS.FEE_CONFIG, this.feeConfig);

    this.logAdminAction(admin, 'UPDATE_FEE_CONFIG', `Updated fees: Reg=${this.feeConfig.registrationFee} XLM`);
    this.emitEvent('Dongle:update_fee_config', { updated_by: admin, isEnabled: this.feeConfig.isFeeEnabled });
    return this.feeConfig;
  }

  public getAdmins(): string[] {
    return this.adminList;
  }

  public addAdmin(caller: string, newAdmin: string): void {
    if (!this.adminList.includes(caller)) throw new Error('ErrUnauthorized: Admin required');
    if (!this.adminList.includes(newAdmin)) {
      this.adminList.push(newAdmin);
      this.save(STORAGE_KEYS.ADMIN_LIST, this.adminList);
      this.logAdminAction(caller, 'ADD_ADMIN', `Added new admin ${newAdmin}`);
      this.emitEvent('Dongle:add_admin', { admin: newAdmin, added_by: caller });
    }
  }

  // --- DATA ACCESSORS ---
  public getVerifications(): VerificationRequest[] {
    return this.verifications;
  }

  public getDisputes(): DuplicateDispute[] {
    return this.disputes;
  }

  public getEvents(): ContractEvent[] {
    return this.events;
  }

  public getAdminLogs(): AdminActionLog[] {
    return this.adminLogs;
  }

  public getStats(): ContractStats {
    return {
      totalProjects: this.projects.length,
      totalVerified: this.projects.filter((p) => p.verified).length,
      totalReviews: this.reviews.length,
      totalCollections: this.collections.length,
      totalAdmins: this.adminList.length,
      totalDisputes: this.disputes.filter((d) => d.status === 'open').length,
      contractTtlLedgers: 3100000,
      currentLedger: this.ledgerSeq,
    };
  }
}

export const contractEngine = new ContractEngineService();
