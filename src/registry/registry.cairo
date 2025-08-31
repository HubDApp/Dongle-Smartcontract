use starknet::ContractAddress;
use starknet::get_caller_address;
use starknet::storage::*;
use super::super::interfaces::{IRegistry, Dapp, DappAdded, DappUpdated, DappClaimed, OwnershipTransferred, FlagsUpdated};
use super::super::cid::Cid;
use super::super::access::{only_admin, only_owner_or_admin};
use super::super::pagination::{safe_slice, validate_pagination};

#[starknet::contract]
pub mod Registry {
    use super::*;

    #[storage]
    pub struct Storage {
        pub dapps: Map<u32, Dapp>,
        pub dapp_count: u32,
        pub admin: ContractAddress,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        DappAdded: DappAdded,
        DappUpdated: DappUpdated,
        DappClaimed: DappClaimed,
        OwnershipTransferred: OwnershipTransferred,
        FlagsUpdated: FlagsUpdated,
    }

    #[constructor]
    fn constructor(ref self: ContractState, admin: ContractAddress) {
        self.admin.write(admin);
        self.dapp_count.write(0);
    }

    #[abi(embed_v0)]
    impl RegistryImpl of IRegistry<ContractState> {
        /// Add a new dApp to the registry (admin or owner)
        fn add_dapp(
            ref self: ContractState,
            name: felt252,
            primary_contract: ContractAddress,
            category: u8,
            metadata_cid: Cid
        ) -> u32 {
            // Admin or owner can add dApps
            // For new dApps, the caller becomes the owner
            let caller = get_caller_address();
            let dapp_id = self.dapp_count.read() + 1;
            
            let dapp = Dapp {
                id: dapp_id,
                name,
                primary_contract,
                category,
                metadata_cid,
                owner: caller,
                claimed: true,
                verified: false,
                featured: false,
            };
            
            self.dapps.write(dapp_id, dapp);
            self.dapp_count.write(dapp_id);
            
            self.emit(Event::DappAdded(DappAdded {
                dapp_id,
                owner: caller,
                name,
                category,
                metadata_cid,
            }));
            
            dapp_id
        }

        /// Update dApp metadata (only owner or admin)
        fn update_dapp(ref self: ContractState, dapp_id: u32, metadata_cid: Cid) {
            let dapp = self.dapps.read(dapp_id);
            only_owner_or_admin(dapp.owner, self.admin.read());
            
            let mut updated_dapp = dapp;
            updated_dapp.metadata_cid = metadata_cid;
            self.dapps.write(dapp_id, updated_dapp);
            
            self.emit(Event::DappUpdated(DappUpdated {
                dapp_id,
                owner: dapp.owner,
                metadata_cid,
            }));
        }

        /// Set claimed status (admin only)
        fn set_claimed(ref self: ContractState, dapp_id: u32, claimed: bool) {
            only_admin(self.admin.read());
            
            let mut dapp = self.dapps.read(dapp_id);
            dapp.claimed = claimed;
            self.dapps.write(dapp_id, dapp);
        }

        /// Claim an unclaimed dApp
        fn claim_dapp(ref self: ContractState, dapp_id: u32) {
            let dapp = self.dapps.read(dapp_id);
            assert(!dapp.claimed, 'ERR_ALREADY_CLAIMED');
            
            let caller = get_caller_address();
            let mut updated_dapp = dapp;
            updated_dapp.owner = caller;
            updated_dapp.claimed = true;
            self.dapps.write(dapp_id, updated_dapp);
            
            self.emit(Event::DappClaimed(DappClaimed {
                dapp_id,
                owner: caller,
            }));
        }

        /// Transfer ownership (only owner or admin)
        fn transfer_ownership(ref self: ContractState, dapp_id: u32, new_owner: ContractAddress) {
            let dapp = self.dapps.read(dapp_id);
            only_owner_or_admin(dapp.owner, self.admin.read());
            
            let old_owner = dapp.owner;
            let mut updated_dapp = dapp;
            updated_dapp.owner = new_owner;
            self.dapps.write(dapp_id, updated_dapp);
            
            self.emit(Event::OwnershipTransferred(OwnershipTransferred {
                dapp_id,
                old_owner,
                new_owner,
            }));
        }

        /// Set verified status (admin only)
        fn set_verified(ref self: ContractState, dapp_id: u32, verified: bool) {
            only_admin(self.admin.read());
            
            let mut dapp = self.dapps.read(dapp_id);
            dapp.verified = verified;
            self.dapps.write(dapp_id, dapp);
            
            self.emit(Event::FlagsUpdated(FlagsUpdated {
                dapp_id,
                verified,
                featured: dapp.featured,
            }));
        }

        /// Set featured status (admin only)
        fn set_featured(ref self: ContractState, dapp_id: u32, featured: bool) {
            only_admin(self.admin.read());
            
            let mut dapp = self.dapps.read(dapp_id);
            dapp.featured = featured;
            self.dapps.write(dapp_id, dapp);
            
            self.emit(Event::FlagsUpdated(FlagsUpdated {
                dapp_id,
                verified: dapp.verified,
                featured,
            }));
        }

        /// Get a specific dApp
        fn get_dapp(self: @ContractState, dapp_id: u32) -> Dapp {
            self.dapps.read(dapp_id)
        }

        /// List dApps with pagination
        fn list_dapps(self: @ContractState, offset: u32, limit: u32) -> Array<Dapp> {
            validate_pagination(offset, limit);
            
            let count = self.dapp_count.read();
            let mut dapps = array![];
            
            let mut i = 1;
            loop {
                if i > count {
                    break;
                };
                dapps.append(self.dapps.read(i));
                i += 1;
            };
            
            safe_slice(dapps, offset, limit)
        }
    }
}
