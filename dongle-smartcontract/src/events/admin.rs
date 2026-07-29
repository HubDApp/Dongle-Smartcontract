use crate::types::AdminActionType;
use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminAddedEvent {
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRemovedEvent {
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractPausedEvent {
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractUnpausedEvent {
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReservedNameAddedEvent {
    pub name: String,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReservedNameRemovedEvent {
    pub name: String,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockActionScheduledEvent {
    pub action_id: u64,
    pub admin: Address,
    pub action_type: AdminActionType,
    pub execution_timestamp: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockActionCancelledEvent {
    pub action_id: u64,
    pub admin: Address,
    pub action_type: AdminActionType,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockActionExecutedEvent {
    pub action_id: u64,
    pub admin: Address,
    pub action_type: AdminActionType,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionCreatedEvent {
    pub collection_id: u64,
    pub name: String,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionUpdatedEvent {
    pub collection_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionDeletedEvent {
    pub collection_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectAddedToCollectionEvent {
    pub collection_id: u64,
    pub project_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjRemovedFromCollectionEvent {
    pub collection_id: u64,
    pub project_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

pub fn publish_admin_added_event(env: &Env, admin: Address) {
    let event_data = AdminAddedEvent {
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events()
        .publish((symbol_short!("ADMIN"), symbol_short!("ADDED")), event_data);
}

pub fn publish_admin_removed_event(env: &Env, admin: Address) {
    let event_data = AdminRemovedEvent {
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("ADMIN"), symbol_short!("REMOVED")),
        event_data,
    );
}

pub fn publish_contract_paused_event(env: &Env, admin: Address) {
    let event_data = ContractPausedEvent {
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("CONTRACT"), symbol_short!("PAUSED")),
        event_data,
    );
}

pub fn publish_contract_unpaused_event(env: &Env, admin: Address) {
    let event_data = ContractUnpausedEvent {
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("CONTRACT"), symbol_short!("UNPAUSED")),
        event_data,
    );
}

pub fn publish_reserved_name_added_event(env: &Env, name: String, admin: Address) {
    let event_data = ReservedNameAddedEvent {
        name,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("CONFIG"), symbol_short!("RSVD_ADD")),
        event_data,
    );
}

pub fn publish_reserved_name_removed_event(env: &Env, name: String, admin: Address) {
    let event_data = ReservedNameRemovedEvent {
        name,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("CONFIG"), symbol_short!("RSVD_REM")),
        event_data,
    );
}

pub fn publish_timelock_action_scheduled_event(
    env: &Env,
    action_id: u64,
    admin: Address,
    action_type: AdminActionType,
    execution_timestamp: u64,
) {
    let event_data = TimelockActionScheduledEvent {
        action_id,
        admin,
        action_type,
        execution_timestamp,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("TIMELOCK"), symbol_short!("SCHEDULE")),
        event_data,
    );
}

pub fn publish_timelock_action_cancelled_event(
    env: &Env,
    action_id: u64,
    admin: Address,
    action_type: AdminActionType,
) {
    let event_data = TimelockActionCancelledEvent {
        action_id,
        admin,
        action_type,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("TIMELOCK"), symbol_short!("CANCEL")),
        event_data,
    );
}

pub fn publish_timelock_action_executed_event(
    env: &Env,
    action_id: u64,
    admin: Address,
    action_type: AdminActionType,
) {
    let event_data = TimelockActionExecutedEvent {
        action_id,
        admin,
        action_type,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("TIMELOCK"), symbol_short!("EXECUTE")),
        event_data,
    );
}

pub fn publish_collection_created_event(
    env: &Env,
    collection_id: u64,
    name: String,
    admin: Address,
) {
    let event_data = CollectionCreatedEvent {
        collection_id,
        name,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("COLLECT"),
            symbol_short!("CREATED"),
            collection_id,
        ),
        event_data,
    );
}

pub fn publish_collection_updated_event(env: &Env, collection_id: u64, admin: Address) {
    let event_data = CollectionUpdatedEvent {
        collection_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("COLLECT"),
            symbol_short!("UPDATED"),
            collection_id,
        ),
        event_data,
    );
}

pub fn publish_collection_deleted_event(env: &Env, collection_id: u64, admin: Address) {
    let event_data = CollectionDeletedEvent {
        collection_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("COLLECT"),
            symbol_short!("DELETED"),
            collection_id,
        ),
        event_data,
    );
}

pub fn publish_project_added_to_collection_event(
    env: &Env,
    collection_id: u64,
    project_id: u64,
    admin: Address,
) {
    let event_data = ProjectAddedToCollectionEvent {
        collection_id,
        project_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("COLLECT"),
            symbol_short!("ADDED"),
            collection_id,
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_removed_from_collection_event(
    env: &Env,
    collection_id: u64,
    project_id: u64,
    admin: Address,
) {
    let event_data = ProjRemovedFromCollectionEvent {
        collection_id,
        project_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("COLLECT"),
            symbol_short!("REMOVED"),
            collection_id,
            project_id,
        ),
        event_data,
    );
}
