#[cfg(test)]
mod test {
    use crate::DongleContract;
    use crate::DongleContractClient;
    use crate::errors::ContractError;
    use crate::types::{VerificationStatus};
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

    fn setup(env: &Env) -> (DongleContractClient<'_>, Address, Address) {
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin);
        (client, admin, Address::generate(env))
    }

    #[test]
    fn test_verification_lifecycle() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, owner) = setup(&env);

        let project_name = String::from_str(&env, "Project X");
        let project_id = client.register_project(
            &owner,
            &project_name,
            &String::from_str(&env, "Description... Description... Description... Description..."),
            &String::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        // 1. Initially unverified
        let project = client.get_project(&project_id).unwrap();
        assert_eq!(project.verification_status, VerificationStatus::Unverified);

        // 2. Set fee (using admin)
        client.set_fee(&admin, &None, &100, &admin);

        // 3. Pay fee (using owner)
        // Note: native transfer case returns error in current implementation because token address is None
        // Let's use a mock token for real test
        let token_admin = Address::generate(&env);
        let token_address = env.register_stellar_asset_contract(token_admin);
        client.set_fee(&admin, &Some(token_address.clone()), &100, &admin);
        
        // Mock token balance for owner
        let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
        token_client.mint(&owner, &1000);
        
        client.pay_fee(&owner, &project_id, &Some(token_address.clone()));

        // 4. Request verification
        client.request_verification(&project_id, &owner, &String::from_str(&env, "ipfs://evidence"));
        
        let project = client.get_project(&project_id).unwrap();
        assert_eq!(project.verification_status, VerificationStatus::Pending);

        // 5. Approve verification (using admin)
        client.approve_verification(&project_id, &admin);
        
        let project = client.get_project(&project_id).unwrap();
        assert_eq!(project.verification_status, VerificationStatus::Verified);
    }

    #[test]
    fn test_reject_verification() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, owner) = setup(&env);

        let project_id = client.register_project(
            &owner,
            &String::from_str(&env, "Project Y"),
            &String::from_str(&env, "Description... Description... Description... Description..."),
            &String::from_str(&env, "NFT"),
            &None,
            &None,
            &None,
        );

        // Set fee and pay
        let token_admin = Address::generate(&env);
        let token_address = env.register_stellar_asset_contract(token_admin);
        let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
        token_client.mint(&owner, &100);
        client.set_fee(&admin, &Some(token_address.clone()), &100, &admin);
        client.pay_fee(&owner, &project_id, &Some(token_address));

        client.request_verification(&project_id, &owner, &String::from_str(&env, "ipfs://evidence"));

        // Reject
        client.reject_verification(&project_id, &admin);
        
        let project = client.get_project(&project_id).unwrap();
        assert_eq!(project.verification_status, VerificationStatus::Rejected);
    }
}
