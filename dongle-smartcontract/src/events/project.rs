use soroban_sdk::{contracttype, symbol_short, Address, Env, Map, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRegisteredEvent {
    pub project_id: u64,
    pub owner: Address,
    pub name: String,
    pub category: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUpdatedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectArchivedEvent {
    pub project_id: u64,
    pub archived_by: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReactivatedEvent {
    pub project_id: u64,
    pub caller: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectOwnershipTransferredEvent {
    pub project_id: u64,
    pub caller: Address,
    pub old_owner: Address,
    pub new_owner: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReportedEvent {
    pub project_id: u64,
    pub reporter: Address,
    pub reason_cid: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReportsClearedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectTagsUpdatedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub tags: Option<Vec<String>>,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSocialLinksUpdatedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub social_links: Option<Map<String, String>>,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReviewsEnabledSetEvent {
    pub project_id: u64,
    pub caller: Address,
    pub enabled: bool,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectClaimableSetEvent {
    pub project_id: u64,
    pub caller: Address,
    pub claimable: bool,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectFollowedEvent {
    pub project_id: u64,
    pub follower: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUnfollowedEvent {
    pub project_id: u64,
    pub follower: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectBookmarkedEvent {
    pub project_id: u64,
    pub user: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUnbookmarkedEvent {
    pub project_id: u64,
    pub user: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectEndorsedEvent {
    pub project_id: u64,
    pub user: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUnendorsedEvent {
    pub project_id: u64,
    pub user: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectMaintainerAddedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub maintainer: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectMaintainerRemovedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub maintainer: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRequestSubmittedEvent {
    pub claim_request_id: u64,
    pub project_id: u64,
    pub claimant: Address,
    pub proof_cid: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRequestApprovedEvent {
    pub claim_request_id: u64,
    pub project_id: u64,
    pub claimant: Address,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRequestRejectedEvent {
    pub claim_request_id: u64,
    pub project_id: u64,
    pub claimant: Address,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractClaimSubmittedEvent {
    pub project_id: u64,
    pub contract_address: String,
    pub claimant: Address,
    pub proof_cid: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractClaimApprovedEvent {
    pub project_id: u64,
    pub contract_address: String,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractClaimRejectedEvent {
    pub project_id: u64,
    pub contract_address: String,
    pub admin: Address,
    pub timestamp: u64,
}

pub fn publish_project_registered_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    name: String,
    category: String,
) {
    let event_data = ProjectRegisteredEvent {
        project_id,
        owner,
        name,
        category,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("CREATED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_updated_event(env: &Env, project_id: u64, owner: Address) {
    let event_data = ProjectUpdatedEvent {
        project_id,
        owner,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("UPDATED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_archived_event(env: &Env, project_id: u64, archived_by: Address) {
    let event_data = ProjectArchivedEvent {
        project_id,
        archived_by,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("ARCHIVED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_reactivated_event(env: &Env, project_id: u64, caller: Address) {
    let event_data = ProjectReactivatedEvent {
        project_id,
        caller,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("RESTORED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_reported_event(
    env: &Env,
    project_id: u64,
    reporter: Address,
    reason_cid: String,
) {
    let event_data = ProjectReportedEvent {
        project_id,
        reporter,
        reason_cid,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("REPORTED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_reports_cleared_event(env: &Env, project_id: u64, admin: Address) {
    let event_data = ProjectReportsClearedEvent {
        project_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("RPCLEARED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_tags_updated_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    tags: Option<Vec<String>>,
) {
    let event_data = ProjectTagsUpdatedEvent {
        project_id,
        owner,
        tags,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("PROJECT"), symbol_short!("TAGS"), project_id),
        event_data,
    );
}

pub fn publish_project_social_links_updated_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    social_links: Option<Map<String, String>>,
) {
    let event_data = ProjectSocialLinksUpdatedEvent {
        project_id,
        owner,
        social_links,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("SOCIAL"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_ownership_transferred_event(
    env: &Env,
    project_id: u64,
    caller: Address,
    old_owner: Address,
    new_owner: Address,
) {
    let event_data = ProjectOwnershipTransferredEvent {
        project_id,
        caller,
        old_owner,
        new_owner,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("TRANSFER"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_reviews_enabled_set_event(
    env: &Env,
    project_id: u64,
    caller: Address,
    enabled: bool,
) {
    let event_data = ProjectReviewsEnabledSetEvent {
        project_id,
        caller,
        enabled,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("REVIEWS"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_claimable_set_event(
    env: &Env,
    project_id: u64,
    caller: Address,
    claimable: bool,
) {
    let event_data = ProjectClaimableSetEvent {
        project_id,
        caller,
        claimable,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("CLAIMABLE"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_claim_request_submitted_event(
    env: &Env,
    claim_request_id: u64,
    project_id: u64,
    claimant: Address,
    proof_cid: String,
) {
    let event_data = ClaimRequestSubmittedEvent {
        claim_request_id,
        project_id,
        claimant: claimant.clone(),
        proof_cid,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CLAIM"),
            symbol_short!("SUBMITTED"),
            project_id,
            claimant,
        ),
        event_data,
    );
}

pub fn publish_claim_request_approved_event(
    env: &Env,
    claim_request_id: u64,
    project_id: u64,
    claimant: Address,
    admin: Address,
) {
    let event_data = ClaimRequestApprovedEvent {
        claim_request_id,
        project_id,
        claimant: claimant.clone(),
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CLAIM"),
            symbol_short!("APPROVED"),
            project_id,
            claimant,
        ),
        event_data,
    );
}

pub fn publish_claim_request_rejected_event(
    env: &Env,
    claim_request_id: u64,
    project_id: u64,
    claimant: Address,
    admin: Address,
) {
    let event_data = ClaimRequestRejectedEvent {
        claim_request_id,
        project_id,
        claimant: claimant.clone(),
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CLAIM"),
            symbol_short!("REJECTED"),
            project_id,
            claimant,
        ),
        event_data,
    );
}

pub fn publish_contract_claim_submitted_event(
    env: &Env,
    project_id: u64,
    contract_address: String,
    claimant: Address,
    proof_cid: String,
) {
    let event_data = ContractClaimSubmittedEvent {
        project_id,
        contract_address: contract_address.clone(),
        claimant: claimant.clone(),
        proof_cid,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CCLAIM"),
            symbol_short!("SUBMITTED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_contract_claim_approved_event(
    env: &Env,
    project_id: u64,
    contract_address: String,
    admin: Address,
) {
    let event_data = ContractClaimApprovedEvent {
        project_id,
        contract_address: contract_address.clone(),
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CCLAIM"),
            symbol_short!("APPROVED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_contract_claim_rejected_event(
    env: &Env,
    project_id: u64,
    contract_address: String,
    admin: Address,
) {
    let event_data = ContractClaimRejectedEvent {
        project_id,
        contract_address: contract_address.clone(),
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CCLAIM"),
            symbol_short!("REJECTED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_maintainer_added_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    maintainer: Address,
) {
    let event_data = ProjectMaintainerAddedEvent {
        project_id,
        owner,
        maintainer: maintainer.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("M_ADDED"),
            project_id,
            maintainer,
        ),
        event_data,
    );
}

pub fn publish_project_maintainer_removed_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    maintainer: Address,
) {
    let event_data = ProjectMaintainerRemovedEvent {
        project_id,
        owner,
        maintainer: maintainer.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("M_REMOVED"),
            project_id,
            maintainer,
        ),
        event_data,
    );
}

pub fn publish_project_followed_event(env: &Env, project_id: u64, follower: Address) {
    let event_data = ProjectFollowedEvent {
        project_id,
        follower: follower.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("FOLLOWED"),
            project_id,
            follower,
        ),
        event_data,
    );
}

pub fn publish_project_unfollowed_event(env: &Env, project_id: u64, follower: Address) {
    let event_data = ProjectUnfollowedEvent {
        project_id,
        follower: follower.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("UNFOLLOW"),
            project_id,
            follower,
        ),
        event_data,
    );
}

pub fn publish_project_bookmarked_event(env: &Env, project_id: u64, user: Address) {
    let event_data = ProjectBookmarkedEvent {
        project_id,
        user: user.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("BOOKMARK"),
            project_id,
            user,
        ),
        event_data,
    );
}

pub fn publish_project_unbookmarked_event(env: &Env, project_id: u64, user: Address) {
    let event_data = ProjectUnbookmarkedEvent {
        project_id,
        user: user.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("UNBOOKMK"),
            project_id,
            user,
        ),
        event_data,
    );
}

pub fn publish_project_endorsed_event(env: &Env, project_id: u64, user: Address) {
    let event_data = ProjectEndorsedEvent {
        project_id,
        user: user.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("ENDORSE"),
            project_id,
            user,
        ),
        event_data,
    );
}

pub fn publish_project_unendorsed_event(env: &Env, project_id: u64, user: Address) {
    let event_data = ProjectUnendorsedEvent {
        project_id,
        user: user.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("UNENDOR"),
            project_id,
            user,
        ),
        event_data,
    );
}

pub fn publish_project_linked_event(
    env: &Env,
    project_id: u64,
    linked_project_id: u64,
    owner: Address,
) {
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("LINKED"),
            project_id,
        ),
        (linked_project_id, owner, env.ledger().timestamp()),
    );
}

pub fn publish_project_unlinked_event(
    env: &Env,
    project_id: u64,
    linked_project_id: u64,
    owner: Address,
) {
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("UNLINKED"),
            project_id,
        ),
        (linked_project_id, owner, env.ledger().timestamp()),
    );
}

pub fn publish_featured_project_event(env: &Env, project_id: u64, featured: bool, admin: Address) {
    let event_data = crate::types::FeaturedProjectEvent {
        project_id,
        featured,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("FEATURED"),
            project_id,
        ),
        event_data,
    );
}
