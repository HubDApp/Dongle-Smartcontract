use starknet::ContractAddress;
use super::cid::Cid;

// Registry Interface
#[starknet::interface]
pub trait IRegistry<TContractState> {
    // Write functions
    fn add_dapp(
        ref self: TContractState, 
        name: felt252, 
        primary_contract: ContractAddress, 
        category: u8, 
        metadata_cid: Cid
    ) -> u32;
    
    fn update_dapp(ref self: TContractState, dapp_id: u32, metadata_cid: Cid);
    fn set_claimed(ref self: TContractState, dapp_id: u32, claimed: bool);
    fn claim_dapp(ref self: TContractState, dapp_id: u32);
    fn transfer_ownership(ref self: TContractState, dapp_id: u32, new_owner: ContractAddress);
    fn set_verified(ref self: TContractState, dapp_id: u32, verified: bool);
    fn set_featured(ref self: TContractState, dapp_id: u32, featured: bool);
    
    // Read functions
    fn get_dapp(self: @TContractState, dapp_id: u32) -> Dapp;
    fn list_dapps(self: @TContractState, offset: u32, limit: u32) -> Array<Dapp>;
}

// Ratings Interface
#[starknet::interface]
pub trait IRatings<TContractState> {
    // Write functions
    fn add_review(
        ref self: TContractState, 
        dapp_id: u32, 
        stars: u8, 
        review_cid: Cid
    ) -> u32;
    
    fn update_review(ref self: TContractState, review_id: u32, stars: u8, review_cid: Cid);
    
    // Read functions
    fn get_average(self: @TContractState, dapp_id: u32) -> (u16, u32);
    fn list_reviews(self: @TContractState, dapp_id: u32, offset: u32, limit: u32) -> Array<Review>;
}

// FeeManager Interface
#[starknet::interface]
pub trait IFeeManager<TContractState> {
    fn pay_verification_fee(ref self: TContractState, project_id: u32, payer: ContractAddress);
    fn set_fee_config(
        ref self: TContractState, 
        amount: u256, 
        token_type: TokenType, 
        token_address: ContractAddress
    );
    fn set_treasury(ref self: TContractState, new_treasury: ContractAddress);
    fn is_fee_paid(self: @TContractState, project_id: u32, payer: ContractAddress) -> bool;
    fn get_fee_config(self: @TContractState) -> FeeConfig;
    fn get_treasury(self: @TContractState) -> ContractAddress;
    fn get_admin(self: @TContractState) -> ContractAddress;
}

// VerificationRegistry Interface
#[starknet::interface]
pub trait IVerificationRegistry<TContractState> {
    fn request_verification(ref self: TContractState, project_id: u32, evidence_cid: Cid);
    fn approve_verification(ref self: TContractState, project_id: u32);
    fn reject_verification(ref self: TContractState, project_id: u32);
    fn suspend_verification(ref self: TContractState, project_id: u32);
    fn revoke_verification(ref self: TContractState, project_id: u32);
    fn get_verification_request(self: @TContractState, project_id: u32) -> VerificationRequest;
    fn get_verification_status(self: @TContractState, project_id: u32) -> VerificationStatus;
}

// Registry structs
#[derive(Drop, Serde, starknet::Store, Copy)]
pub struct Dapp {
    pub id: u32,
    pub name: felt252,
    pub primary_contract: ContractAddress,
    pub category: u8,
    pub metadata_cid: Cid,
    pub owner: ContractAddress,
    pub claimed: bool,
    pub verified: bool,
    pub featured: bool,
}

// Ratings structs
#[derive(Drop, Serde, starknet::Store, Copy)]
pub struct Review {
    pub id: u32,
    pub dapp_id: u32,
    pub reviewer: ContractAddress,
    pub stars: u8,
    pub review_cid: Cid,
}

// FeeManager structs and enums
#[derive(Drop, Serde, starknet::Store, Copy, PartialEq)]
#[allow(starknet::store_no_default_variant)]
pub enum TokenType {
    STRK,
    ERC20,
}

#[derive(Drop, Serde, starknet::Store)]
pub struct FeeConfig {
    pub amount: u256,
    pub token_type: TokenType,
    pub token_address: ContractAddress, // For ERC20 tokens
}

// VerificationRegistry structs and enums
#[derive(Drop, Serde, starknet::Store, Copy, PartialEq)]
#[allow(starknet::store_no_default_variant)]
pub enum VerificationStatus {
    None,
    Pending,
    Verified,
    Rejected,
    Suspended,
    Revoked,
}

#[derive(Drop, Serde, starknet::Store)]
pub struct VerificationRequest {
    pub project_id: u32,
    pub requester: ContractAddress,
    pub evidence_cid: Cid,
    pub status: VerificationStatus,
    pub timestamp: u64,
}

// Registry events
#[derive(Drop, starknet::Event)]
pub struct DappAdded {
    pub dapp_id: u32,
    pub owner: ContractAddress,
    pub name: felt252,
    pub category: u8,
    pub metadata_cid: Cid,
}

#[derive(Drop, starknet::Event)]
pub struct DappUpdated {
    pub dapp_id: u32,
    pub owner: ContractAddress,
    pub metadata_cid: Cid,
}

#[derive(Drop, starknet::Event)]
pub struct DappClaimed {
    pub dapp_id: u32,
    pub owner: ContractAddress,
}

#[derive(Drop, starknet::Event)]
pub struct OwnershipTransferred {
    pub dapp_id: u32,
    pub old_owner: ContractAddress,
    pub new_owner: ContractAddress,
}

#[derive(Drop, starknet::Event)]
pub struct FlagsUpdated {
    pub dapp_id: u32,
    pub verified: bool,
    pub featured: bool,
}

// Ratings events
#[derive(Drop, starknet::Event)]
pub struct ReviewAdded {
    pub review_id: u32,
    pub dapp_id: u32,
    pub reviewer: ContractAddress,
    pub stars: u8,
    pub review_cid: Cid,
}

#[derive(Drop, starknet::Event)]
pub struct ReviewUpdated {
    pub review_id: u32,
    pub dapp_id: u32,
    pub reviewer: ContractAddress,
    pub stars: u8,
    pub review_cid: Cid,
}

// FeeManager events
#[derive(Drop, starknet::Event)]
pub struct FeePaid {
    pub project_id: u32,
    pub payer: ContractAddress,
    pub amount: u256,
    pub token_type: TokenType,
    pub token_address: ContractAddress,
}

#[derive(Drop, starknet::Event)]
pub struct FeeConfigUpdated {
    pub old_amount: u256,
    pub new_amount: u256,
    pub token_type: TokenType,
    pub token_address: ContractAddress,
}

#[derive(Drop, starknet::Event)]
pub struct TreasuryUpdated {
    pub old_treasury: ContractAddress,
    pub new_treasury: ContractAddress,
}

// VerificationRegistry events
#[derive(Drop, starknet::Event)]
pub struct VerificationRequested {
    pub project_id: u32,
    pub requester: ContractAddress,
    pub evidence_cid: Cid,
}

#[derive(Drop, starknet::Event)]
pub struct VerificationStatusChanged {
    pub project_id: u32,
    pub old_status: VerificationStatus,
    pub new_status: VerificationStatus,
    pub admin: ContractAddress,
} 
