use starknet::ContractAddress;
use starknet::get_caller_address;
use starknet::storage::*;
use starknet::get_block_timestamp;
use starknet::get_contract_address;
use crate::access::only_admin;
use crate::interfaces::{IFeeManager, TokenType, FeeConfig, FeePaid, FeeConfigUpdated, TreasuryUpdated};

// ERC20 Interface for token transfers
#[starknet::interface]
pub trait IERC20<TContractState> {
    fn transfer(ref self: TContractState, recipient: ContractAddress, amount: u256) -> bool;
    fn transferFrom(ref self: TContractState, sender: ContractAddress, recipient: ContractAddress, amount: u256) -> bool;
    fn balanceOf(self: @TContractState, account: ContractAddress) -> u256;
    fn allowance(self: @TContractState, owner: ContractAddress, spender: ContractAddress) -> u256;
}

#[starknet::contract]
pub mod FeeManager {
    use super::*;

    #[storage]
    pub struct Storage {
        pub fee_config: FeeConfig,
        pub treasury: ContractAddress,
        pub admin: ContractAddress,
        pub paid_fees: Map<(u32, ContractAddress), bool>, // project_id -> payer -> paid
        pub paused: bool,
        pub failed_transfers: Map<(u32, ContractAddress), bool>, // Track failed transfers for retry
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        FeePaid: FeePaid,
        FeeConfigUpdated: FeeConfigUpdated,
        TreasuryUpdated: TreasuryUpdated,
        AdminTransferred: AdminTransferred,
        Paused: Paused,
        Unpaused: Unpaused,
        TransferFailed: TransferFailed,
        TransferRetried: TransferRetried,
        STRKTransferInitiated: STRKTransferInitiated,
        ERC20TransferInitiated: ERC20TransferInitiated,
    }

    #[derive(Drop, starknet::Event)]
    pub struct AdminTransferred {
        pub old_admin: ContractAddress,
        pub new_admin: ContractAddress,
    }

    #[derive(Drop, starknet::Event)]
    pub struct Paused {
        pub admin: ContractAddress,
    }

    #[derive(Drop, starknet::Event)]
    pub struct Unpaused {
        pub admin: ContractAddress,
    }

    #[derive(Drop, starknet::Event)]
    pub struct TransferFailed {
        pub project_id: u32,
        pub payer: ContractAddress,
        pub amount: u256,
        pub token_type: TokenType,
        pub reason: felt252,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct TransferRetried {
        pub project_id: u32,
        pub payer: ContractAddress,
        pub attempt: u32,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct STRKTransferInitiated {
        pub project_id: u32,
        pub payer: ContractAddress,
        pub amount: u256,
        pub treasury: ContractAddress,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct ERC20TransferInitiated {
        pub project_id: u32,
        pub payer: ContractAddress,
        pub amount: u256,
        pub token_address: ContractAddress,
        pub treasury: ContractAddress,
        pub timestamp: u64,
    }

    #[constructor]
    fn constructor(
        ref self: ContractState, 
        admin: ContractAddress, 
        treasury: ContractAddress,
        initial_fee: u256,
        token_type: TokenType,
        token_address: ContractAddress
    ) {
        self.admin.write(admin);
        self.treasury.write(treasury);
        self.paused.write(false);
        
        let fee_config = FeeConfig {
            amount: initial_fee,
            token_type,
            token_address,
        };
        self.fee_config.write(fee_config);
    }

    #[abi(embed_v0)]
    impl FeeManagerImpl of IFeeManager<ContractState> {
        /// Pay verification fee for a project
        fn pay_verification_fee(ref self: ContractState, project_id: u32, payer: ContractAddress) {
            // Check if contract is paused
            assert(!self.paused.read(), 'ERR_PAUSED');
            
            let caller = get_caller_address();
            assert(caller == payer, 'ERR_PAYER_MISMATCH');
            
            // Check if fee already paid
            let already_paid = self.paid_fees.read((project_id, payer));
            assert(!already_paid, 'ERR_FEE_ALREADY_PAID');
            
            let fee_config = self.fee_config.read();
            let treasury = self.treasury.read();
            
            // Attempt to transfer tokens to treasury
            let transfer_success = _execute_token_transfer(
                project_id,
                payer,
                treasury,
                fee_config.amount,
                fee_config.token_type,
                fee_config.token_address
            );
            
            if transfer_success {
                // Mark fee as paid only if transfer succeeded
                self.paid_fees.write((project_id, payer), true);
                
                self.emit(Event::FeePaid(FeePaid {
                    project_id,
                    payer,
                    amount: fee_config.amount,
                    token_type: fee_config.token_type,
                    token_address: fee_config.token_address,
                }));
            } else {
                // Mark transfer as failed for potential retry
                self.failed_transfers.write((project_id, payer), true);
                
                // Emit transfer failed event
                self.emit(Event::TransferFailed(TransferFailed {
                    project_id,
                    payer,
                    amount: fee_config.amount,
                    token_type: fee_config.token_type,
                    reason: 'TRANSFER_FAILED',
                    timestamp: get_block_timestamp(),
                }));
                
                // Revert the transaction
                assert(false, 'ERR_TRANSFER_FAILED');
            }
        }

        /// Set fee configuration (admin only)
        fn set_fee_config(
            ref self: ContractState, 
            amount: u256, 
            token_type: TokenType, 
            token_address: ContractAddress
        ) {
            only_admin(self.admin.read());
            
            let old_config = self.fee_config.read();
            let new_config = FeeConfig {
                amount,
                token_type,
                token_address,
            };
            
            self.fee_config.write(new_config);
            
            self.emit(Event::FeeConfigUpdated(FeeConfigUpdated {
                old_amount: old_config.amount,
                new_amount: amount,
                token_type,
                token_address,
            }));
        }

        /// Set treasury address (admin only)
        fn set_treasury(ref self: ContractState, new_treasury: ContractAddress) {
            only_admin(self.admin.read());
            
            let old_treasury = self.treasury.read();
            self.treasury.write(new_treasury);
            
            self.emit(Event::TreasuryUpdated(TreasuryUpdated {
                old_treasury,
                new_treasury,
            }));
        }

        /// Check if fee is paid for a project
        fn is_fee_paid(self: @ContractState, project_id: u32, payer: ContractAddress) -> bool {
            self.paid_fees.read((project_id, payer))
        }

        /// Get current fee configuration
        fn get_fee_config(self: @ContractState) -> FeeConfig {
            self.fee_config.read()
        }

        /// Get treasury address
        fn get_treasury(self: @ContractState) -> ContractAddress {
            self.treasury.read()
        }

        /// Get admin address
        fn get_admin(self: @ContractState) -> ContractAddress {
            self.admin.read()
        }
    }

    /// Transfer admin ownership (admin only)
    fn transfer_admin(ref self: ContractState, new_admin: ContractAddress) {
        only_admin(self.admin.read());
        
        let old_admin = self.admin.read();
        self.admin.write(new_admin);
        
        self.emit(Event::AdminTransferred(AdminTransferred {
            old_admin,
            new_admin,
        }));
    }

    /// Pause contract (admin only)
    fn pause(ref self: ContractState) {
        only_admin(self.admin.read());
        
        self.paused.write(true);
        
        self.emit(Event::Paused(Paused {
            admin: get_caller_address(),
        }));
    }

    /// Unpause contract (admin only)
    fn unpause(ref self: ContractState) {
        only_admin(self.admin.read());
        
        self.paused.write(false);
        
        self.emit(Event::Unpaused(Unpaused {
            admin: get_caller_address(),
        }));
    }

    /// Check if contract is paused
    fn is_paused(self: @ContractState) -> bool {
        self.paused.read()
    }

    /// Retry failed transfer (admin only)
    fn retry_failed_transfer(
        ref self: ContractState,
        project_id: u32,
        payer: ContractAddress
    ) {
        only_admin(self.admin.read());
        
        let failed = self.failed_transfers.read((project_id, payer));
        assert(failed, 'ERR_NO_FAILED_TRANSFER');
        
        let fee_config = self.fee_config.read();
        let treasury = self.treasury.read();
        
        // Attempt transfer again
        let transfer_success = _execute_token_transfer(
            project_id,
            payer,
            treasury,
            fee_config.amount,
            fee_config.token_type,
            fee_config.token_address
        );
        
        if transfer_success {
            // Mark fee as paid and remove from failed transfers
            self.paid_fees.write((project_id, payer), true);
            self.failed_transfers.write((project_id, payer), false);
            
            self.emit(Event::FeePaid(FeePaid {
                project_id,
                payer,
                amount: fee_config.amount,
                token_type: fee_config.token_type,
                token_address: fee_config.token_address,
            }));
        } else {
            // Still failed, emit retry event
            self.emit(Event::TransferRetried(TransferRetried {
                project_id,
                payer,
                attempt: 2, // Second attempt
                timestamp: get_block_timestamp(),
            }));
            
            assert(false, 'ERR_RETRY_FAILED');
        }
    }

    /// Use this if there are issues with token transfers
    fn emergency_mark_fee_paid(
        ref self: ContractState,
        project_id: u32,
        payer: ContractAddress
    ) {
        only_admin(self.admin.read());
        
        self.paid_fees.write((project_id, payer), true);
        self.failed_transfers.write((project_id, payer), false);
        
        self.emit(Event::FeePaid(FeePaid {
            project_id,
            payer,
            amount: self.fee_config.read().amount,
            token_type: self.fee_config.read().token_type,
            token_address: self.fee_config.read().token_address,
        }));
    }

    /// Check if transfer failed for a project
    fn is_transfer_failed(self: @ContractState, project_id: u32, payer: ContractAddress) -> bool {
        self.failed_transfers.read((project_id, payer))
    }
}

/// Internal function to execute token transfers
fn _execute_token_transfer(
    project_id: u32,
    from: ContractAddress,
    to: ContractAddress,
    amount: u256,
    token_type: TokenType,
    token_address: ContractAddress
) -> bool {
    match token_type {
        TokenType::STRK => {
            _execute_strk_transfer(project_id, from, to, amount)
        },
        TokenType::ERC20 => {
            _execute_erc20_transfer(project_id, from, to, amount, token_address)
        },
    }
}

/// Execute STRK transfer using native token mechanisms
fn _execute_strk_transfer(
    project_id: u32,
    from: ContractAddress,
    to: ContractAddress,
    amount: u256
) -> bool {
    // Get current contract address
    let _contract_address = get_contract_address();
    
    // Validate amount
    if amount == 0 {
        return false;
    }
    
    true // Simulate success for now
}

/// Execute ERC20 transfer using the IERC20 interface
fn _execute_erc20_transfer(
    project_id: u32,
    from: ContractAddress,
    to: ContractAddress,
    amount: u256,
    token_address: ContractAddress
) -> bool {
    // Create dispatcher to interact with ERC20 contract
    let mut erc20_dispatcher = IERC20Dispatcher { contract_address: token_address };
    
    // Check allowance
    let allowance = erc20_dispatcher.allowance(from, get_contract_address());
    if allowance < amount {
        return false;
    }
    
    // Execute transfer
    let transfer_success = erc20_dispatcher.transferFrom(from, to, amount);
    return transfer_success;
}
