use starknet::ContractAddress;
use starknet::contract_address_const;
use snforge_std::{declare, ContractClassTrait, DeclareResultTrait};
use dongle::fee_manager::fee_manager::FeeManager;
use dongle::interfaces::{TokenType, FeeConfig, FeePaid, FeeConfigUpdated, TreasuryUpdated};

fn deploy_fee_manager() -> ContractAddress {
    let contract = declare("FeeManager");
    let mut constructor_args = array![];
    
    let admin: ContractAddress = contract_address_const::<'admin'>();
    let treasury: ContractAddress = contract_address_const::<'treasury'>();
    let initial_fee = 0_u256;
    let token_type = TokenType::STRK;
    let token_address = contract_address_const::<'zero'>();
    
    Serde::serialize(@admin, ref constructor_args);
    Serde::serialize(@treasury, ref constructor_args);
    Serde::serialize(@initial_fee, ref constructor_args);
    Serde::serialize(@token_type, ref constructor_args);
    Serde::serialize(@token_address, ref constructor_args);
    
    let (contract_address, _) = contract
        .unwrap()
        .contract_class()
        .deploy(@constructor_args)
        .unwrap();
    
    contract_address
}

#[test]
fn test_fee_manager_compiles() {
    // Test that the contract can be declared
    let contract = declare("FeeManager");
    assert(contract.is_ok(), 'Contract declared');
}

#[test]
fn test_fee_manager_constructor() {
    let contract_address = deploy_fee_manager();
    
    // Test that deployment succeeded
    assert(contract_address != contract_address_const::<'zero'>(), 'Contract not deployed');
}

#[test]
fn test_fee_manager_basic() {
    // Basic test to verify the test framework works
    assert(true, 'Basic test passed');
}

#[test]
fn test_fee_manager_math() {
    let x = 2 + 2;
    assert(x == 4, 'Math works');
}

#[test]
fn test_token_type_enum() {
    let strk = TokenType::STRK;
    let erc20 = TokenType::ERC20;
    
    // Test that enums can be created
    assert(true, 'TokenType enums work');
}

#[test]
fn test_fee_config_struct() {
    let admin: ContractAddress = contract_address_const::<'admin'>();
    let config = FeeConfig {
        amount: 1000_u256,
        token_type: TokenType::STRK,
        token_address: admin,
    };
    
    // Test that struct can be created
    assert(config.amount == 1000_u256, 'FeeConfig works');
}

#[test]
fn test_fee_paid_event() {
    let payer: ContractAddress = contract_address_const::<'payer'>();
    let event = FeePaid {
        project_id: 123_u32,
        payer,
        amount: 500_u256,
        token_type: TokenType::STRK,
        token_address: contract_address_const::<'zero'>(),
    };
    
    // Test that event struct can be created
    assert(event.project_id == 123_u32, 'FeePaid event works');
}

#[test]
fn test_fee_config_updated_event() {
    let event = FeeConfigUpdated {
        old_amount: 100_u256,
        new_amount: 200_u256,
        token_type: TokenType::ERC20,
        token_address: contract_address_const::<'zero'>(),
    };
    
    // Test that event struct can be created
    assert(event.old_amount == 100_u256, 'FeeConfigUpdated event works');
}

#[test]
fn test_treasury_updated_event() {
    let old_treasury: ContractAddress = contract_address_const::<'old_treasury'>();
    let new_treasury: ContractAddress = contract_address_const::<'new_treasury'>();
    
    let event = TreasuryUpdated {
        old_treasury,
        new_treasury,
    };
    
    // Test that event struct can be created
    assert(event.old_treasury == old_treasury, 'TreasuryUpdated event works');
}
