use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeeOperation {
    Verification,
    Registration,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeSetEvent {
    pub admin: Address,
    pub token: Option<Address>,
    pub verification_fee: u128,
    pub registration_fee: u128,
    pub treasury: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeePaidEvent {
    pub project_id: u64,
    pub payer: Address,
    pub token: Option<Address>,
    pub operation: FeeOperation,
    pub amount: u128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConsumedEvent {
    pub project_id: u64,
    pub caller: Address,
    pub operation: FeeOperation,
    pub amount: u128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeRefundedEvent {
    pub project_id: u64,
    pub request_id: u64,
    pub payer: Address,
    pub amount: u128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeePaymentClearedEvent {
    pub project_id: u64,
    pub payer: Address,
    pub paid_at: u64,
    pub cleared_at: u64,
}

pub fn publish_fee_set_event(
    env: &Env,
    admin: Address,
    token: Option<Address>,
    verification_fee: u128,
    registration_fee: u128,
    treasury: Address,
) {
    let event_data = FeeSetEvent {
        admin,
        token,
        verification_fee,
        registration_fee,
        treasury,
        timestamp: env.ledger().timestamp(),
    };
    env.events()
        .publish((symbol_short!("CONFIG"), symbol_short!("FEE")), event_data);
}

pub fn publish_fee_paid_event(
    env: &Env,
    project_id: u64,
    payer: Address,
    token: Option<Address>,
    operation: FeeOperation,
    amount: u128,
) {
    let event_data = FeePaidEvent {
        project_id,
        payer,
        token,
        operation,
        amount,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("FEE"), symbol_short!("PAID"), project_id),
        event_data,
    );
}

pub fn publish_fee_consumed_event(
    env: &Env,
    project_id: u64,
    caller: Address,
    operation: FeeOperation,
    amount: u128,
) {
    let event_data = FeeConsumedEvent {
        project_id,
        caller,
        operation,
        amount,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("FEE"), symbol_short!("CONSUMED"), project_id),
        event_data,
    );
}

pub fn publish_fee_refunded_event(
    env: &Env,
    project_id: u64,
    request_id: u64,
    payer: Address,
    amount: u128,
) {
    let event_data = FeeRefundedEvent {
        project_id,
        request_id,
        payer,
        amount,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("FEE"), symbol_short!("REFUNDED"), project_id),
        event_data,
    );
}

pub fn publish_fee_payment_cleared_event(
    env: &Env,
    project_id: u64,
    payer: Address,
    paid_at: u64,
    cleared_at: u64,
) {
    let event_data = FeePaymentClearedEvent {
        project_id,
        payer,
        paid_at,
        cleared_at,
    };
    env.events().publish(
        (symbol_short!("FEE"), symbol_short!("CLEARED"), project_id),
        event_data,
    );
}
