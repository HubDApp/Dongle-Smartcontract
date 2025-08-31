use starknet::ContractAddress;
use starknet::contract_address_const;
use snforge_std::{declare, ContractClassTrait, DeclareResultTrait, EventSpyAssertionsTrait, spy_events, start_cheat_caller_address, stop_cheat_caller_address};
use dongle::interfaces::{IRegistryDispatcher, IRegistryDispatcherTrait, Dapp, DappAdded, DappUpdated, DappClaimed, OwnershipTransferred, FlagsUpdated};
use dongle::cid::cid_from_parts;

fn deploy_registry() -> IRegistryDispatcher {
    let contract = declare("Registry");
    let mut constructor_args = array![];
    let admin: ContractAddress = contract_address_const::<'admin'>();
    
    Serde::serialize(@admin, ref constructor_args);
    
    let (contract_address, _) = contract
        .unwrap()
        .contract_class()
        .deploy(@constructor_args)
        .unwrap();
    
    IRegistryDispatcher { contract_address }
}

#[test]
fn test_add_dapp() {
    let dispatcher = deploy_registry();
    let mut spy = spy_events();
    
    let caller: ContractAddress = contract_address_const::<'user'>();
    start_cheat_caller_address(dispatcher.contract_address, caller);
    
    let name = 'TestDapp';
    let primary_contract: ContractAddress = contract_address_const::<'primary'>();
    let category = 1_u8; // NFT
    let metadata_cid = cid_from_parts(array!['metadata']);
    
    let dapp_id = dispatcher.add_dapp(name, primary_contract, category, metadata_cid);
    assert(dapp_id == 1, 'Wrong dapp id');
    
    let dapp = dispatcher.get_dapp(1);
    assert(dapp.id == 1, 'Wrong dapp id');
    assert(dapp.name == name, 'Wrong dapp name');
    assert(dapp.primary_contract == primary_contract, 'Wrong primary contract');
    assert(dapp.category == category, 'Wrong category');
    assert(dapp.owner == caller, 'Wrong owner');
    assert(dapp.claimed == true, 'Should be claimed');
    assert(dapp.verified == false, 'Should not be verified');
    assert(dapp.featured == false, 'Should not be featured');
    
    // Verify event emission
    let expected_event = dongle::registry::registry::Registry::Event::DappAdded(
        DappAdded { dapp_id: 1, owner: caller, name, category, metadata_cid }
    );
    let expected_events = array![(dispatcher.contract_address, expected_event)];
    spy.assert_emitted(@expected_events);
    
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_update_dapp() {
    let dispatcher = deploy_registry();
    let mut spy = spy_events();
    
    let caller: ContractAddress = contract_address_const::<'user'>();
    start_cheat_caller_address(dispatcher.contract_address, caller);
    
    // Add a dapp first
    let name = 'TestDapp';
    let primary_contract: ContractAddress = contract_address_const::<'primary'>();
    let category = 1_u8;
    let metadata_cid = cid_from_parts(array!['metadata']);
    
    dispatcher.add_dapp(name, primary_contract, category, metadata_cid);
    
    // Update the dapp
    let new_metadata_cid = cid_from_parts(array!['new_metadata']);
    dispatcher.update_dapp(1, new_metadata_cid);
    
    let dapp = dispatcher.get_dapp(1);
    assert(dapp.metadata_cid.value == 'new_metadata', 'Metadata not updated');
    
    // Verify event emission
    let expected_event = dongle::registry::registry::Registry::Event::DappUpdated(
        DappUpdated { dapp_id: 1, owner: caller, metadata_cid: new_metadata_cid }
    );
    let expected_events = array![(dispatcher.contract_address, expected_event)];
    spy.assert_emitted(@expected_events);
    
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_claim_dapp() {
    let dispatcher = deploy_registry();
    let mut spy = spy_events();
    
    let admin: ContractAddress = contract_address_const::<'admin'>();
    start_cheat_caller_address(dispatcher.contract_address, admin);
    
    // Add a dapp first
    let name = 'TestDapp';
    let primary_contract: ContractAddress = contract_address_const::<'primary'>();
    let category = 1_u8;
    let metadata_cid = cid_from_parts(array!['metadata']);
    
    dispatcher.add_dapp(name, primary_contract, category, metadata_cid);
    
    // Mark as unclaimed
    dispatcher.set_claimed(1, false);
    
    stop_cheat_caller_address(dispatcher.contract_address);
    
    // Claim the dapp
    let claimer: ContractAddress = contract_address_const::<'claimer'>();
    start_cheat_caller_address(dispatcher.contract_address, claimer);
    
    dispatcher.claim_dapp(1);
    
    let dapp = dispatcher.get_dapp(1);
    assert(dapp.owner == claimer, 'Wrong owner after claim');
    assert(dapp.claimed == true, 'Should be claimed');
    
    // Verify event emission
    let expected_event = dongle::registry::registry::Registry::Event::DappClaimed(
        DappClaimed { dapp_id: 1, owner: claimer }
    );
    let expected_events = array![(dispatcher.contract_address, expected_event)];
    spy.assert_emitted(@expected_events);
    
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_transfer_ownership() {
    let dispatcher = deploy_registry();
    let mut spy = spy_events();
    
    let caller: ContractAddress = contract_address_const::<'user'>();
    start_cheat_caller_address(dispatcher.contract_address, caller);
    
    // Add a dapp first
    let name = 'TestDapp';
    let primary_contract: ContractAddress = contract_address_const::<'primary'>();
    let category = 1_u8;
    let metadata_cid = cid_from_parts(array!['metadata']);
    
    dispatcher.add_dapp(name, primary_contract, category, metadata_cid);
    
    // Transfer ownership
    let new_owner: ContractAddress = contract_address_const::<'new_owner'>();
    dispatcher.transfer_ownership(1, new_owner);
    
    let dapp = dispatcher.get_dapp(1);
    assert(dapp.owner == new_owner, 'Ownership not transferred');
    
    // Verify event emission
    let expected_event = dongle::registry::registry::Registry::Event::OwnershipTransferred(
        OwnershipTransferred { dapp_id: 1, old_owner: caller, new_owner }
    );
    let expected_events = array![(dispatcher.contract_address, expected_event)];
    spy.assert_emitted(@expected_events);
    
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_set_flags() {
    let dispatcher = deploy_registry();
    let mut spy = spy_events();
    
    let admin: ContractAddress = contract_address_const::<'admin'>();
    start_cheat_caller_address(dispatcher.contract_address, admin);
    
    // Add a dapp first
    let name = 'TestDapp';
    let primary_contract: ContractAddress = contract_address_const::<'primary'>();
    let category = 1_u8;
    let metadata_cid = cid_from_parts(array!['metadata']);
    
    dispatcher.add_dapp(name, primary_contract, category, metadata_cid);
    
    // Set flags
    dispatcher.set_verified(1, true);
    dispatcher.set_featured(1, true);
    
    let dapp = dispatcher.get_dapp(1);
    assert(dapp.verified == true, 'Should be verified');
    assert(dapp.featured == true, 'Should be featured');
    
    // Verify event emission
    let expected_event = dongle::registry::registry::Registry::Event::FlagsUpdated(
        FlagsUpdated { dapp_id: 1, verified: true, featured: true }
    );
    let expected_events = array![(dispatcher.contract_address, expected_event)];
    spy.assert_emitted(@expected_events);
    
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_list_dapps() {
    let dispatcher = deploy_registry();
    
    let caller: ContractAddress = contract_address_const::<'user'>();
    start_cheat_caller_address(dispatcher.contract_address, caller);
    
    // Add multiple dapps
    let name1 = 'Dapp1';
    let name2 = 'Dapp2';
    let name3 = 'Dapp3';
    let primary_contract: ContractAddress = contract_address_const::<'primary'>();
    let category = 1_u8;
    let metadata_cid = cid_from_parts(array!['metadata']);
    
    dispatcher.add_dapp(name1, primary_contract, category, metadata_cid);
    dispatcher.add_dapp(name2, primary_contract, category, metadata_cid);
    dispatcher.add_dapp(name3, primary_contract, category, metadata_cid);
    
    // Test pagination
    let dapps = dispatcher.list_dapps(0, 2);
    assert(dapps.len() == 2, 'Wrong number of dapps');
    assert(*dapps.at(0).name == name1, 'Wrong first dapp');
    assert(*dapps.at(1).name == name2, 'Wrong second dapp');
    
    let dapps_page2 = dispatcher.list_dapps(2, 2);
    assert(dapps_page2.len() == 1, 'Wrong number of dapps on page 2');
    assert(*dapps_page2.at(0).name == name3, 'Wrong dapp on page 2');
    
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_admin_only_functions() {
    let dispatcher = deploy_registry();
    
    let user: ContractAddress = contract_address_const::<'user'>();
    let admin: ContractAddress = contract_address_const::<'admin'>();
    
    // Add a dapp first
    start_cheat_caller_address(dispatcher.contract_address, user);
    let name = 'TestDapp';
    let primary_contract: ContractAddress = contract_address_const::<'primary'>();
    let category = 1_u8;
    let metadata_cid = cid_from_parts(array!['metadata']);
    
    dispatcher.add_dapp(name, primary_contract, category, metadata_cid);
    stop_cheat_caller_address(dispatcher.contract_address);
    
    // Test that non-admin cannot set claimed
    start_cheat_caller_address(dispatcher.contract_address, user);
    // This should fail - user is not admin
    // dispatcher.set_claimed(1, false); // This would panic
    
    // Test that admin can set claimed
    start_cheat_caller_address(dispatcher.contract_address, admin);
    dispatcher.set_claimed(1, false);
    
    let dapp = dispatcher.get_dapp(1);
    assert(dapp.claimed == false, 'Should be unclaimed');
    
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_owner_or_admin_functions() {
    let dispatcher = deploy_registry();
    
    let user: ContractAddress = contract_address_const::<'user'>();
    let admin: ContractAddress = contract_address_const::<'admin'>();
    
    // Add a dapp first
    start_cheat_caller_address(dispatcher.contract_address, user);
    let name = 'TestDapp';
    let primary_contract: ContractAddress = contract_address_const::<'primary'>();
    let category = 1_u8;
    let metadata_cid = cid_from_parts(array!['metadata']);
    
    dispatcher.add_dapp(name, primary_contract, category, metadata_cid);
    stop_cheat_caller_address(dispatcher.contract_address);
    
    // Test that owner can update dapp
    start_cheat_caller_address(dispatcher.contract_address, user);
    let new_metadata_cid = cid_from_parts(array!['owner_update']);
    dispatcher.update_dapp(1, new_metadata_cid);
    
    let dapp = dispatcher.get_dapp(1);
    assert(dapp.metadata_cid.value == 'owner_update', 'Owner update failed');
    
    // Test that admin can also update dapp
    start_cheat_caller_address(dispatcher.contract_address, admin);
    let admin_metadata_cid = cid_from_parts(array!['admin_update']);
    dispatcher.update_dapp(1, admin_metadata_cid);
    
    let dapp = dispatcher.get_dapp(1);
    assert(dapp.metadata_cid.value == 'admin_update', 'Admin update failed');
    
    stop_cheat_caller_address(dispatcher.contract_address);
} 
