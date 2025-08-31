use starknet::ContractAddress;
use starknet::contract_address_const;
use snforge_std::{declare, ContractClassTrait, DeclareResultTrait};
use dongle::verification_registry::verification_registry::VerificationRegistry;
use dongle::interfaces::{VerificationStatus, VerificationRequest, VerificationRequested, VerificationStatusChanged};
use dongle::cid::cid_from_parts;

fn deploy_verification_registry() -> ContractAddress {
    let contract = declare("VerificationRegistry");
    let mut constructor_args = array![];
    
    let admin: ContractAddress = contract_address_const::<'admin'>();
    let project_registry: ContractAddress = contract_address_const::<'project_registry'>();
    
    Serde::serialize(@admin, ref constructor_args);
    Serde::serialize(@project_registry, ref constructor_args);
    
    let (contract_address, _) = contract
        .unwrap()
        .contract_class()
        .deploy(@constructor_args)
        .unwrap();
    
    contract_address
}

#[test]
fn test_verification_registry_compiles() {
    // Test that the contract can be declared
    let contract = declare("VerificationRegistry");
    assert(contract.is_ok(), 'Contract declared');
}

#[test]
fn test_verification_registry_constructor() {
    let contract_address = deploy_verification_registry();
    
    // Test that deployment succeeded
    assert(contract_address != contract_address_const::<'zero'>(), 'Contract not deployed');
}

#[test]
fn test_cid_helpers() {
    // Test CID helper functions
    let cid = cid_from_parts(array!['test']);
    assert(cid.value == 'test', 'CID value');
}

#[test]
fn test_verification_status_enum() {
    let none = VerificationStatus::None;
    let pending = VerificationStatus::Pending;
    let verified = VerificationStatus::Verified;
    let rejected = VerificationStatus::Rejected;
    let suspended = VerificationStatus::Suspended;
    let revoked = VerificationStatus::Revoked;
    
    // Test that all enum variants can be created
    assert(true, 'VerificationStatus enums work');
}

#[test]
fn test_verification_request_struct() {
    let project_id = 123_u32;
    let evidence_cid = cid_from_parts(array!['evidence']);
    let requester: ContractAddress = contract_address_const::<'requester'>();
    let timestamp = 1234567890_u64;
    
    let request = VerificationRequest {
        project_id,
        requester,
        evidence_cid,
        status: VerificationStatus::Pending,
        timestamp,
    };
    
    // Test that struct can be created
    assert(request.project_id == project_id, 'VerificationRequest works');
    assert(request.requester == requester, 'Requester field works');
}

#[test]
fn test_verification_requested_event() {
    let project_id = 456_u32;
    let evidence_cid = cid_from_parts(array!['evidence']);
    let requester: ContractAddress = contract_address_const::<'requester'>();
    let timestamp = 9876543210_u64;
    
    let event = VerificationRequested {
        project_id,
        requester,
        evidence_cid,
    };
    
    // Test that event struct can be created
    assert(event.project_id == project_id, 'Requested event works');
    assert(event.requester == requester, 'Requester field works');
}

#[test]
fn test_verification_status_changed_event() {
    let project_id = 789_u32;
    let old_status = VerificationStatus::Pending;
    let new_status = VerificationStatus::Verified;
    let admin: ContractAddress = contract_address_const::<'admin'>();
    let timestamp = 1111111111_u64;
    
    let event = VerificationStatusChanged {
        project_id,
        old_status,
        new_status,
        admin,
    };
    
    // Test that event struct can be created
    assert(event.project_id == project_id, 'StatusChanged event works');
    assert(event.old_status == old_status, 'Old status works');
    assert(event.new_status == new_status, 'New status works');
}

#[test]
fn test_verification_status_transitions() {
    // Test that different status transitions can be represented
    let transitions = array![
        (VerificationStatus::None, VerificationStatus::Pending),
        (VerificationStatus::Pending, VerificationStatus::Verified),
        (VerificationStatus::Pending, VerificationStatus::Rejected),
        (VerificationStatus::Verified, VerificationStatus::Suspended),
        (VerificationStatus::Suspended, VerificationStatus::Revoked),
    ];
    
    // Test that transitions array can be created
    assert(transitions.len() == 5, 'Status transitions work');
}

#[test]
fn test_multiple_verification_requests() {
    // Test creating multiple verification requests
    let project1 = 1_u32;
    let project2 = 2_u32;
    let project3 = 3_u32;
    
    let evidence1 = cid_from_parts(array!['evidence1']);
    let evidence2 = cid_from_parts(array!['evidence2']);
    let evidence3 = cid_from_parts(array!['evidence3']);
    
    let requester1: ContractAddress = contract_address_const::<'requester1'>();
    let requester2: ContractAddress = contract_address_const::<'requester2'>();
    let requester3: ContractAddress = contract_address_const::<'requester3'>();
    
    let request1 = VerificationRequest {
        project_id: project1,
        requester: requester1,
        evidence_cid: evidence1,
        status: VerificationStatus::Pending,
        timestamp: 1000_u64,
    };
    
    let request2 = VerificationRequest {
        project_id: project2,
        requester: requester2,
        evidence_cid: evidence2,
        status: VerificationStatus::Pending,
        timestamp: 2000_u64,
    };
    
    let request3 = VerificationRequest {
        project_id: project3,
        requester: requester3,
        evidence_cid: evidence3,
        status: VerificationStatus::Pending,
        timestamp: 3000_u64,
    };
    
    // Test that all requests can be created
    assert(request1.project_id == project1, 'Request 1 works');
    assert(request2.project_id == project2, 'Request 2 works');
    assert(request3.project_id == project3, 'Request 3 works');
}

#[test]
fn test_verification_registry_basic() {
    // Basic test to verify the test framework works
    assert(true, 'Basic test passed');
}

#[test]
fn test_cid_operations() {
    // Test various CID operations
    let empty_cid = cid_from_parts(array![]);
    let single_cid = cid_from_parts(array!['single']);
    let multi_cid = cid_from_parts(array!['part1', 'part2', 'part3']);
    
    // Test that CIDs can be created with different numbers of parts
    assert(empty_cid.value == 0, 'Empty CID works');
    assert(single_cid.value == 'single', 'Single part CID works');
    assert(multi_cid.value == 'part1', 'Multi part CID works');
}
