use starknet::ContractAddress;
use starknet::get_caller_address;
use starknet::storage::*;
use crate::cid::Cid;
use crate::access::only_admin;
use crate::interfaces::{IVerificationRegistry, VerificationStatus, VerificationRequest, VerificationRequested, VerificationStatusChanged};

#[starknet::contract]
pub mod VerificationRegistry {
    use super::*;

    #[storage]
    pub struct Storage {
        pub verification_requests: Map<u32, VerificationRequest>,
        pub admin: ContractAddress,
        pub project_registry: ContractAddress,
        pub paused: bool,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        VerificationRequested: VerificationRequested,
        VerificationStatusChanged: VerificationStatusChanged,
        AdminTransferred: AdminTransferred,
        Paused: Paused,
        Unpaused: Unpaused,
        RequestWithdrawn: RequestWithdrawn,
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
    pub struct RequestWithdrawn {
        pub project_id: u32,
        pub requester: ContractAddress,
    }

    #[constructor]
    fn constructor(ref self: ContractState, admin: ContractAddress, project_registry: ContractAddress) {
        self.admin.write(admin);
        self.project_registry.write(project_registry);
        self.paused.write(false);
    }

    #[abi(embed_v0)]
    impl VerificationRegistryImpl of IVerificationRegistry<ContractState> {
        /// Request verification for a project
        fn request_verification(ref self: ContractState, project_id: u32, evidence_cid: Cid) {
            // Check if contract is paused
            assert(!self.paused.read(), 'ERR_PAUSED');
            
            let caller = get_caller_address();
        
            
            // Check if project already has a verification request
            let existing_request = self.verification_requests.read(project_id);
            assert(existing_request.status == VerificationStatus::None, 'ERR_ALREADY_REQUESTED');
            
            let request = VerificationRequest {
                project_id,
                requester: caller,
                evidence_cid,
                status: VerificationStatus::Pending,
                timestamp: starknet::get_block_timestamp(),
            };
            
            self.verification_requests.write(project_id, request);
            
            self.emit(Event::VerificationRequested(VerificationRequested {
                project_id,
                requester: caller,
                evidence_cid,
            }));
        }

        /// Approve verification (admin only)
        fn approve_verification(ref self: ContractState, project_id: u32) {
            only_admin(self.admin.read());
            
            let mut request = self.verification_requests.read(project_id);
            let old_status = request.status;
            
            assert(request.status == VerificationStatus::Pending, 'ERR_NOT_PENDING');
            
            request.status = VerificationStatus::Verified;
            self.verification_requests.write(project_id, request);
            
            self.emit(Event::VerificationStatusChanged(VerificationStatusChanged {
                project_id,
                old_status,
                new_status: VerificationStatus::Verified,
                admin: get_caller_address(),
            }));
        }

        /// Reject verification (admin only)
        fn reject_verification(ref self: ContractState, project_id: u32) {
            only_admin(self.admin.read());
            
            let mut request = self.verification_requests.read(project_id);
            let old_status = request.status;
            
            assert(request.status == VerificationStatus::Pending, 'ERR_NOT_PENDING');
            
            request.status = VerificationStatus::Rejected;
            self.verification_requests.write(project_id, request);
            
            self.emit(Event::VerificationStatusChanged(VerificationStatusChanged {
                project_id,
                old_status,
                new_status: VerificationStatus::Rejected,
                admin: get_caller_address(),
            }));
        }

        /// Suspend verification (admin only)
        fn suspend_verification(ref self: ContractState, project_id: u32) {
            only_admin(self.admin.read());
            
            let mut request = self.verification_requests.read(project_id);
            let old_status = request.status;
            
            assert(request.status == VerificationStatus::Verified, 'ERR_NOT_VERIFIED');
            
            request.status = VerificationStatus::Suspended;
            self.verification_requests.write(project_id, request);
            
            self.emit(Event::VerificationStatusChanged(VerificationStatusChanged {
                project_id,
                old_status,
                new_status: VerificationStatus::Suspended,
                admin: get_caller_address(),
            }));
        }

        /// Revoke verification (admin only)
        fn revoke_verification(ref self: ContractState, project_id: u32) {
            only_admin(self.admin.read());
            
            let mut request = self.verification_requests.read(project_id);
            let old_status = request.status;
            
            assert(request.status == VerificationStatus::Verified || request.status == VerificationStatus::Suspended, 'ERR_NOT_VERIFIED_OR_SUSPENDED');
            
            request.status = VerificationStatus::Revoked;
            self.verification_requests.write(project_id, request);
            
            self.emit(Event::VerificationStatusChanged(VerificationStatusChanged {
                project_id,
                old_status,
                new_status: VerificationStatus::Revoked,
                admin: get_caller_address(),
            }));
        }

        /// Get verification request for a project
        fn get_verification_request(self: @ContractState, project_id: u32) -> VerificationRequest {
            self.verification_requests.read(project_id)
        }

        /// Get verification status for a project
        fn get_verification_status(self: @ContractState, project_id: u32) -> VerificationStatus {
            let request = self.verification_requests.read(project_id);
            request.status
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

    /// Withdraw verification request (only original requester)
    fn withdraw_request(ref self: ContractState, project_id: u32) {
        let caller = get_caller_address();
        let mut request = self.verification_requests.read(project_id);
        
        assert(request.requester == caller, 'ERR_NOT_REQUESTER');
        assert(request.status == VerificationStatus::Pending, 'ERR_NOT_PENDING');
        
        // Reset to None status
        request.status = VerificationStatus::None;
        self.verification_requests.write(project_id, request);
        
        self.emit(Event::RequestWithdrawn(RequestWithdrawn {
            project_id,
            requester: caller,
        }));
    }

    /// Check if contract is paused
    fn is_paused(self: @ContractState) -> bool {
        self.paused.read()
    }

    /// Get project registry address
    fn get_project_registry(self: @ContractState) -> ContractAddress {
        self.project_registry.read()
    }
}
