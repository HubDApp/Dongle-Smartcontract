import {
  Project,
  Review,
  Collection,
  VerificationRequest,
  DuplicateDispute,
  ContractEvent,
  AdminActionLog,
  FeeConfig,
  UserAccount,
} from '../types';

export const TESTNET_CONTRACT_ID = 'CCWUXOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N73';
export const TESTNET_DEPLOYER = 'GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N';

export const MOCK_USERS: UserAccount[] = [
  {
    address: 'GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N',
    name: 'Alice (Contract SuperAdmin)',
    role: 'admin',
  },
  {
    address: 'GCLAUDE47SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQPROJ',
    name: 'Bob (DeFi Builder)',
    role: 'owner',
  },
  {
    address: 'GCHARLIE99SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZREV',
    name: 'Charlie (Ecosystem Reviewer)',
    role: 'reviewer',
  },
];

export const INITIAL_PROJECTS: Project[] = [
  {
    id: 1,
    slug: 'dongle-defi-explorer',
    name: 'Dongle DeFi Explorer',
    description: 'A discovery layer and analytics dashboard for Stellar DeFi protocols. On-chain registry with rating aggregation.',
    website: 'https://dongle.example',
    repository: 'https://github.com/HubDApp/Dongle-Smartcontract',
    documentation: 'https://github.com/HubDApp/Dongle-Smartcontract/blob/main/README.md',
    category: 'DeFi',
    tags: ['stellar', 'soroban', 'discovery', 'analytics'],
    owner: 'GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N',
    maintainers: [
      { name: 'Core Team', email: 'team@dongle.example', url: 'https://dongle.example/team', role: 'owner' }
    ],
    metadataCid: 'QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG',
    logoCid: 'QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG',
    bannerCid: 'QmBannerExample00000000000000000000000000000001',
    verified: true,
    verificationBadge: 'gold_partner',
    verificationExpiresAt: Date.now() + 300 * 86400 * 1000,
    featured: true,
    featuredRank: 1,
    ratingAverage: 4.8,
    ratingCount: 5,
    reviewCount: 5,
    status: 'active',
    createdAt: Date.now() - 60 * 86400 * 1000,
    ttlLedgers: 520000,
    securityContact: {
      contact: 'security@dongle.example',
      proofCid: 'QmSecurityProof000000000000000000000000000001',
      verified: true,
    },
    dependencies: [
      { name: 'Soroban SDK', version: '20.0.0', type: 'Smart Contract' },
      { name: 'Stellar SDK', version: '11.2.0', type: 'Frontend' },
    ],
  },
  {
    id: 2,
    slug: 'soroban-amm-swap',
    name: 'Soroban Automated Market Maker',
    description: 'Constant product automated market maker (AMM) enabling decentralized token swaps and liquidity pools on Stellar.',
    website: 'https://soroban-amm.org',
    repository: 'https://github.com/soroban-apps/amm',
    category: 'DeFi',
    tags: ['amm', 'dex', 'liquidity-pools', 'soroban'],
    owner: 'GCLAUDE47SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZQPROJ',
    maintainers: [
      { name: 'DeFi Devs', email: 'dev@soroban-amm.org', role: 'maintainer' }
    ],
    metadataCid: 'QmAmmMetadata000000000000000000000000000000002',
    verified: true,
    verificationBadge: 'audited_security',
    verificationExpiresAt: Date.now() + 180 * 86400 * 1000,
    featured: true,
    featuredRank: 2,
    ratingAverage: 4.6,
    ratingCount: 8,
    reviewCount: 8,
    status: 'active',
    createdAt: Date.now() - 45 * 86400 * 1000,
    ttlLedgers: 480000,
    securityContact: {
      contact: 'audit@certik.example',
      proofCid: 'QmCertikAuditReport00000000000000000000000002',
      verified: true,
    },
  },
  {
    id: 3,
    slug: 'stellar-anchor-connect',
    name: 'AnchorConnect SDK',
    description: 'Unified SEP-24 and SEP-31 bridge framework for fiat on/off ramps across Latin America and Europe.',
    website: 'https://anchorconnect.io',
    repository: 'https://github.com/anchor-connect/sdk',
    category: 'Infrastructure',
    tags: ['anchors', 'sep24', 'sep31', 'fiat-ramp'],
    owner: 'GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N',
    maintainers: [
      { name: 'Bridge Infra Lead', email: 'support@anchorconnect.io', role: 'owner' }
    ],
    metadataCid: 'QmAnchorMetadata000000000000000000000000000003',
    verified: true,
    verificationBadge: 'verified',
    verificationExpiresAt: Date.now() + 90 * 86400 * 1000,
    featured: false,
    ratingAverage: 4.3,
    ratingCount: 3,
    reviewCount: 3,
    status: 'active',
    createdAt: Date.now() - 30 * 86400 * 1000,
    ttlLedgers: 320000,
  },
  {
    id: 4,
    slug: 'stellar-dao-governance',
    name: 'Soroban Governance Vaults',
    description: 'On-chain DAO proposal creation, quadratic voting, and timelock treasury execution contract system.',
    website: 'https://stellar-dao.org',
    repository: 'https://github.com/stellar-dao/governance',
    category: 'Governance',
    tags: ['dao', 'governance', 'voting', 'treasury'],
    owner: 'GCLAUDE47SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZQPROJ',
    maintainers: [
      { name: 'DAO Committee', email: 'gov@stellar-dao.org', role: 'owner' }
    ],
    metadataCid: 'QmDaoMetadata000000000000000000000000000000004',
    verified: false,
    verificationBadge: 'none',
    featured: false,
    ratingAverage: 4.0,
    ratingCount: 2,
    reviewCount: 2,
    status: 'active',
    createdAt: Date.now() - 15 * 86400 * 1000,
    ttlLedgers: 210000,
  },
  {
    id: 5,
    slug: 'cross-chain-bridge-v1',
    name: 'Soroban Cross-Chain Relay (Legacy)',
    description: 'Experimental message relayer contract for EVM to Soroban proof verification. Marked as claimable.',
    website: 'https://relay.example',
    category: 'Bridges',
    tags: ['bridge', 'cross-chain', 'experimental'],
    owner: 'GCHARLIE99SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47TZREV',
    maintainers: [],
    metadataCid: 'QmBridgeMetadata0000000000000000000000000005',
    verified: false,
    verificationBadge: 'none',
    featured: false,
    ratingAverage: 3.2,
    ratingCount: 1,
    reviewCount: 1,
    status: 'claimable',
    createdAt: Date.now() - 120 * 86400 * 1000,
    ttlLedgers: 90000,
  }
];

export const INITIAL_REVIEWS: Review[] = [
  {
    id: 1,
    projectId: 1,
    reviewer: 'GCHARLIE99SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47TZREV',
    rating: 5,
    title: 'Essential Soroban Project Discovery Tool',
    comment: 'The rating aggregation and verified badge system make it super clear which Soroban projects have security audits and active maintainers.',
    cid: 'QmReviewCid00000000000000000000000000000000001',
    isHidden: false,
    isReported: false,
    createdAt: Date.now() - 10 * 86400 * 1000,
  },
  {
    id: 2,
    projectId: 1,
    reviewer: 'GCLAUDE47SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47TZQPROJ',
    rating: 5,
    title: 'Great integration docs and IPFS schema',
    comment: 'Extremely clean JSON schema validation for project metadata CIDs. Makes indexer parsing effortless.',
    cid: 'QmReviewCid00000000000000000000000000000000002',
    isHidden: false,
    isReported: false,
    createdAt: Date.now() - 5 * 86400 * 1000,
  },
  {
    id: 3,
    projectId: 2,
    reviewer: 'GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N',
    rating: 5,
    title: 'Rock solid AMM implementation',
    comment: 'Gas optimization is top notch and security audit proof CID is verified on-chain.',
    isHidden: false,
    isReported: false,
    createdAt: Date.now() - 12 * 86400 * 1000,
  }
];

export const INITIAL_COLLECTIONS: Collection[] = [
  {
    id: 1,
    name: 'DeFi Ecosystem Essentials',
    description: 'High liquidity protocol building blocks on Soroban testnet & mainnet.',
    category: 'DeFi',
    projectIds: [1, 2],
    creator: 'GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N',
    isFeatured: true,
    createdAt: Date.now() - 30 * 86400 * 1000,
  },
  {
    id: 2,
    name: 'Security Audited Protocols',
    description: 'Projects with verified security contacts and active third-party audit reports.',
    category: 'Security',
    projectIds: [1, 2, 3],
    creator: 'GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N',
    isFeatured: true,
    createdAt: Date.now() - 20 * 86400 * 1000,
  }
];

export const INITIAL_VERIFICATION_REQUESTS: VerificationRequest[] = [
  {
    id: 1,
    projectId: 4,
    requester: 'GCLAUDE47SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZQPROJ',
    badgeLevel: 'verified',
    status: 'pending',
    notes: 'Submitted Github repository proof and active maintainer contacts.',
    requestedAt: Date.now() - 2 * 86400 * 1000,
  }
];

export const INITIAL_DISPUTES: DuplicateDispute[] = [
  {
    id: 1,
    projectId: 5,
    duplicateOfProjectId: 3,
    reporter: 'GCHARLIE99SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47TZREV',
    reason: 'Project #5 appears to be an abandoned duplicate of AnchorConnect SDK with identical descriptions.',
    status: 'open',
    createdAt: Date.now() - 3 * 86400 * 1000,
  }
];

export const INITIAL_FEE_CONFIG: FeeConfig = {
  feeToken: 'CDLZFC3SYJYD3O2O2Z5P3XF2NEMT39OJWN3BDG3247V7OVDJ3Z4R3V47', // Native/SAC
  registrationFee: 10, // 10 XLM
  verificationFee: 50, // 50 XLM
  featureFee: 100, // 100 XLM
  reviewFee: 0,
  isFeeEnabled: true,
};

export const INITIAL_ADMIN_LOGS: AdminActionLog[] = [
  {
    id: 1,
    admin: 'GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N',
    action: 'INITIALIZE_CONTRACT',
    details: 'Contract deployed and initialized on Stellar Testnet.',
    timestamp: Date.now() - 60 * 86400 * 1000,
  },
  {
    id: 2,
    admin: 'GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N',
    action: 'APPROVE_VERIFICATION',
    details: 'Approved Gold Partner badge for Project #1 (Dongle DeFi Explorer).',
    timestamp: Date.now() - 40 * 86400 * 1000,
  }
];

export const INITIAL_EVENTS: ContractEvent[] = [
  {
    id: 'evt-001',
    topic: 'Dongle:initialize',
    data: { admin: TESTNET_DEPLOYER, fee_token: INITIAL_FEE_CONFIG.feeToken },
    timestamp: Date.now() - 60 * 86400 * 1000,
    ledgerSequence: 5410023,
    contractId: TESTNET_CONTRACT_ID,
  },
  {
    id: 'evt-002',
    topic: 'Dongle:register_project',
    data: { project_id: 1, slug: 'dongle-defi-explorer', owner: TESTNET_DEPLOYER },
    timestamp: Date.now() - 60 * 86400 * 1000,
    ledgerSequence: 5410025,
    contractId: TESTNET_CONTRACT_ID,
  },
  {
    id: 'evt-003',
    topic: 'Dongle:approve_verification',
    data: { project_id: 1, badge: 'gold_partner', expires: Date.now() + 300 * 86400 * 1000 },
    timestamp: Date.now() - 40 * 86400 * 1000,
    ledgerSequence: 5489100,
    contractId: TESTNET_CONTRACT_ID,
  }
];
