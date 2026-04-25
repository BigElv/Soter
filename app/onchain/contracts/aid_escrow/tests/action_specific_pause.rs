#![cfg(test)]

use aid_escrow::{AidEscrow, AidEscrowClient, Error, PackageStatus};
use soroban_sdk::{
    Address, Env,
    testutils::{Address as _, Ledger},
    token::{StellarAssetClient, TokenClient},
};

fn setup() -> (Env, AidEscrowClient<'static>, Address, Address, TokenClient<'static>, StellarAssetClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token_admin = Address::generate(&env);
    
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = TokenClient::new(&env, &token_contract.address());
    let token_admin_client = StellarAssetClient::new(&env, &token_contract.address());

    let contract_id = env.register(AidEscrow, ());
    let client = AidEscrowClient::new(&env, &contract_id);

    // Initialize contract
    client.init(&admin);

    // Mint and fund
    token_admin_client.mint(&admin, &10_000);
    client.fund(&token_client.address, &admin, &5000);

    (env, client, admin, recipient, token_client, token_admin_client)
}

// ==================== CREATE PAUSE TESTS ====================

#[test]
fn test_pause_create_blocks_package_creation() {
    let (_env, client, admin, recipient, token_client, _token_admin_client) = setup();

    // Pause create
    client.pause_create(&admin);
    assert!(client.is_create_paused());

    // Attempt to create package should fail
    let expires_at = 86400;
    let result = client.try_create_package(&admin, &1, &recipient, &1000, &token_client.address, &expires_at);
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    let _ = err.downcast::<Error>().unwrap();
}

#[test]
fn test_pause_create_blocks_batch_creation() {
    let (_env, client, admin, recipient, _token_client, _token_admin_client) = setup();

    // Pause create
    client.pause_create(&admin);
    assert!(client.is_create_paused());

    // Attempt to batch create should fail
    let recipients = soroban_sdk::Vec::from_array(&client.env, [recipient.clone()]);
    let amounts = soroban_sdk::Vec::from_array(&client.env, [1000i128]);
    let result = client.try_batch_create_packages(&admin, &recipients, &amounts, &token_client.address, &86400);
    
    assert!(result.is_err());
}

#[test]
fn test_unpause_create_restores_package_creation() {
    let (_env, client, admin, recipient, token_client, _token_admin_client) = setup();

    // Pause and unpause
    client.pause_create(&admin);
    assert!(client.is_create_paused());
    
    client.unpause_create(&admin);
    assert!(!client.is_create_paused());

    // Create package should succeed
    let expires_at = 86400;
    let pkg_id = client.create_package(&admin, &1, &recipient, &1000, &token_client.address, &expires_at);
    assert_eq!(pkg_id, 1);
}

#[test]
fn test_pause_create_does_not_affect_claim() {
    let (_env, client, admin, recipient, token_client, _token_admin_client) = setup();

    // Create package first
    let expires_at = 86400;
    client.create_package(&admin, &1, &recipient, &1000, &token_client.address, &expires_at);

    // Pause create
    client.pause_create(&admin);
    assert!(client.is_create_paused());

    // Claim should still work
    client.claim(&1);
    
    let package = client.get_package(&1);
    assert_eq!(package.status, PackageStatus::Claimed);
}

#[test]
fn test_pause_create_does_not_affect_withdraw() {
    let (_env, client, admin, _recipient, token_client, _token_admin_client) = setup();

    // Pause create
    client.pause_create(&admin);
    assert!(client.is_create_paused());

    // Withdraw should still work (there's 5000 surplus)
    client.withdraw_surplus(&admin, &1000, &token_client.address);
    
    let balance = token_client.balance(&client.address);
    assert_eq!(balance, 4000);
}

// ==================== CLAIM PAUSE TESTS ====================

#[test]
fn test_pause_claim_blocks_claiming() {
    let (_env, client, admin, recipient, token_client, _token_admin_client) = setup();

    // Create package
    let expires_at = 86400;
    client.create_package(&admin, &1, &recipient, &1000, &token_client.address, &expires_at);

    // Pause claim
    client.pause_claim(&admin);
    assert!(client.is_claim_paused());

    // Attempt to claim should fail
    let result = client.try_claim(&1);
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    let _ = err.downcast::<Error>().unwrap();
}

#[test]
fn test_unpause_claim_restores_claiming() {
    let (_env, client, admin, recipient, token_client, _token_admin_client) = setup();

    // Create package
    let expires_at = 86400;
    client.create_package(&admin, &1, &recipient, &1000, &token_client.address, &expires_at);

    // Pause and unpause claim
    client.pause_claim(&admin);
    assert!(client.is_claim_paused());
    
    client.unpause_claim(&admin);
    assert!(!client.is_claim_paused());

    // Claim should succeed
    client.claim(&1);
    
    let package = client.get_package(&1);
    assert_eq!(package.status, PackageStatus::Claimed);
}

#[test]
fn test_pause_claim_does_not_affect_create() {
    let (_env, client, admin, recipient, token_client, _token_admin_client) = setup();

    // Pause claim
    client.pause_claim(&admin);
    assert!(client.is_claim_paused());

    // Create package should still work
    let expires_at = 86400;
    let pkg_id = client.create_package(&admin, &1, &recipient, &1000, &token_client.address, &expires_at);
    assert_eq!(pkg_id, 1);
}

#[test]
fn test_pause_claim_does_not_affect_withdraw() {
    let (_env, client, admin, _recipient, token_client, _token_admin_client) = setup();

    // Pause claim
    client.pause_claim(&admin);
    assert!(client.is_claim_paused());

    // Withdraw should still work
    client.withdraw_surplus(&admin, &1000, &token_client.address);
    
    let balance = token_client.balance(&client.address);
    assert_eq!(balance, 4000);
}

// ==================== WITHDRAW PAUSE TESTS ====================

#[test]
fn test_pause_withdraw_blocks_withdrawal() {
    let (_env, client, admin, _recipient, token_client, _token_admin_client) = setup();

    // Pause withdraw
    client.pause_withdraw(&admin);
    assert!(client.is_withdraw_paused());

    // Attempt to withdraw should fail
    let result = client.try_withdraw_surplus(&admin, &1000, &token_client.address);
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    let _ = err.downcast::<Error>().unwrap();
}

#[test]
fn test_unpause_withdraw_restores_withdrawal() {
    let (_env, client, admin, _recipient, token_client, _token_admin_client) = setup();

    // Pause and unpause withdraw
    client.pause_withdraw(&admin);
    assert!(client.is_withdraw_paused());
    
    client.unpause_withdraw(&admin);
    assert!(!client.is_withdraw_paused());

    // Withdraw should succeed
    client.withdraw_surplus(&admin, &1000, &token_client.address);
    
    let balance = token_client.balance(&client.address);
    assert_eq!(balance, 4000);
}

#[test]
fn test_pause_withdraw_does_not_affect_create() {
    let (_env, client, admin, recipient, token_client, _token_admin_client) = setup();

    // Pause withdraw
    client.pause_withdraw(&admin);
    assert!(client.is_withdraw_paused());

    // Create package should still work
    let expires_at = 86400;
    let pkg_id = client.create_package(&admin, &1, &recipient, &1000, &token_client.address, &expires_at);
    assert_eq!(pkg_id, 1);
}

#[test]
fn test_pause_withdraw_does_not_affect_claim() {
    let (_env, client, admin, recipient, token_client, _token_admin_client) = setup();

    // Create package first
    let expires_at = 86400;
    client.create_package(&admin, &1, &recipient, &1000, &token_client.address, &expires_at);

    // Pause withdraw
    client.pause_withdraw(&admin);
    assert!(client.is_withdraw_paused());

    // Claim should still work
    client.claim(&1);
    
    let package = client.get_package(&1);
    assert_eq!(package.status, PackageStatus::Claimed);
}

// ==================== COMBINED PAUSE TESTS ====================

#[test]
fn test_multiple_actions_can_be_paused_simultaneously() {
    let (_env, client, admin, recipient, token_client, _token_admin_client) = setup();

    // Create a package first
    let expires_at = 86400;
    client.create_package(&admin, &1, &recipient, &1000, &token_client.address, &expires_at);

    // Pause create and claim
    client.pause_create(&admin);
    client.pause_claim(&admin);

    assert!(client.is_create_paused());
    assert!(client.is_claim_paused());
    assert!(!client.is_withdraw_paused());

    // Create should fail
    let result = client.try_create_package(&admin, &2, &recipient, &1000, &token_client.address, &expires_at);
    assert!(result.is_err());

    // Claim should fail
    let result = client.try_claim(&1);
    assert!(result.is_err());

    // Withdraw should succeed
    client.withdraw_surplus(&admin, &1000, &token_client.address);
}

#[test]
fn test_all_actions_paused() {
    let (_env, client, admin, recipient, token_client, _token_admin_client) = setup();

    // Create a package first
    let expires_at = 86400;
    client.create_package(&admin, &1, &recipient, &1000, &token_client.address, &expires_at);

    // Pause all actions
    client.pause_create(&admin);
    client.pause_claim(&admin);
    client.pause_withdraw(&admin);

    assert!(client.is_create_paused());
    assert!(client.is_claim_paused());
    assert!(client.is_withdraw_paused());

    // All actions should fail
    let result = client.try_create_package(&admin, &2, &recipient, &1000, &token_client.address, &expires_at);
    assert!(result.is_err());

    let result = client.try_claim(&1);
    assert!(result.is_err());

    let result = client.try_withdraw_surplus(&admin, &1000, &token_client.address);
    assert!(result.is_err());
}

#[test]
fn test_selective_unpause() {
    let (_env, client, admin, recipient, token_client, _token_admin_client) = setup();

    // Create a package
    let expires_at = 86400;
    client.create_package(&admin, &1, &recipient, &1000, &token_client.address, &expires_at);

    // Pause all
    client.pause_create(&admin);
    client.pause_claim(&admin);
    client.pause_withdraw(&admin);

    // Unpause only claim
    client.unpause_claim(&admin);

    assert!(client.is_create_paused());
    assert!(!client.is_claim_paused());
    assert!(client.is_withdraw_paused());

    // Create should still fail
    let result = client.try_create_package(&admin, &2, &recipient, &1000, &token_client.address, &expires_at);
    assert!(result.is_err());

    // Claim should succeed
    client.claim(&1);
    let package = client.get_package(&1);
    assert_eq!(package.status, PackageStatus::Claimed);

    // Withdraw should still fail
    let result = client.try_withdraw_surplus(&admin, &1000, &token_client.address);
    assert!(result.is_err());
}

// ==================== AUTHORIZATION TESTS ====================

#[test]
fn test_non_admin_cannot_pause_create() {
    let (env, client, admin, _recipient, _token_client, _token_admin_client) = setup();
    
    let non_admin = Address::generate(&env);
    
    // Non-admin trying to pause should fail
    let result = client.try_pause_create(&non_admin);
    assert!(result.is_err());
}

#[test]
fn test_non_admin_cannot_pause_claim() {
    let (env, client, admin, _recipient, _token_client, _token_admin_client) = setup();
    
    let non_admin = Address::generate(&env);
    
    // Non-admin trying to pause should fail
    let result = client.try_pause_claim(&non_admin);
    assert!(result.is_err());
}

#[test]
fn test_non_admin_cannot_pause_withdraw() {
    let (env, client, admin, _recipient, _token_client, _token_admin_client) = setup();
    
    let non_admin = Address::generate(&env);
    
    // Non-admin trying to pause should fail
    let result = client.try_pause_withdraw(&non_admin);
    assert!(result.is_err());
}

#[test]
fn test_non_admin_cannot_unpause() {
    let (env, client, admin, _recipient, _token_client, _token_admin_client) = setup();
    
    let non_admin = Address::generate(&env);
    
    // Admin pauses
    client.pause_create(&admin);
    client.pause_claim(&admin);
    client.pause_withdraw(&admin);

    // Non-admin trying to unpause should fail
    let result = client.try_unpause_create(&non_admin);
    assert!(result.is_err());

    let result = client.try_unpause_claim(&non_admin);
    assert!(result.is_err());

    let result = client.try_unpause_withdraw(&non_admin);
    assert!(result.is_err());
}

// ==================== STATE QUERY TESTS ====================

#[test]
fn test_initial_pause_states_are_false() {
    let (_env, client, _admin, _recipient, _token_client, _token_admin_client) = setup();

    assert!(!client.is_create_paused());
    assert!(!client.is_claim_paused());
    assert!(!client.is_withdraw_paused());
}

#[test]
fn test_pause_states_are_independent() {
    let (_env, client, admin, _recipient, _token_client, _token_admin_client) = setup();

    // Only pause create
    client.pause_create(&admin);
    assert!(client.is_create_paused());
    assert!(!client.is_claim_paused());
    assert!(!client.is_withdraw_paused());

    // Only pause claim
    client.unpause_create(&admin);
    client.pause_claim(&admin);
    assert!(!client.is_create_paused());
    assert!(client.is_claim_paused());
    assert!(!client.is_withdraw_paused());

    // Only pause withdraw
    client.unpause_claim(&admin);
    client.pause_withdraw(&admin);
    assert!(!client.is_create_paused());
    assert!(!client.is_claim_paused());
    assert!(client.is_withdraw_paused());
}
