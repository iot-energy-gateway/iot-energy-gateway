#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events},
    token, Address, BytesN, Env,
};

fn setup() -> (Env, Address, BytesN<32>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let hardware_id = BytesN::from_array(&env, &[0xca; 32]);
    let rate_per_wh: u64 = 10;

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());
    let sac = token::StellarAssetClient::new(&env, &token_contract);
    sac.mint(&user, &1_000_000);
    sac.mint(&admin, &1_000_000);

    let contract_id = env.register_contract(None, EnergyGateway);
    let client = EnergyGatewayClient::new(&env, &contract_id);

    client.initialize(&admin, &hardware_id, &token_contract, &rate_per_wh);

    let token_client = token::Client::new(&env, &token_contract);
    token_client.approve(&user, &contract_id, &1_000_000, &u32::MAX);
    token_client.approve(&admin, &contract_id, &1_000_000, &u32::MAX);

    (env, admin, hardware_id, user, contract_id)
}

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let hardware_id = BytesN::from_array(&env, &[0xca; 32]);
    let token = Address::generate(&env);

    let contract_id = env.register_contract(None, EnergyGateway);
    let client = EnergyGatewayClient::new(&env, &contract_id);

    client.initialize(&admin, &hardware_id, &token, &100);

    assert!(client.get_stream_status(&hardware_id).unwrap());
}

#[test]
fn test_double_initialize_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let hardware_id = BytesN::from_array(&env, &[0xca; 32]);
    let token = Address::generate(&env);

    let contract_id = env.register_contract(None, EnergyGateway);
    let client = EnergyGatewayClient::new(&env, &contract_id);

    client.initialize(&admin, &hardware_id, &token, &100);

    let result = client.try_initialize(&admin, &hardware_id, &token, &100);
    assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn test_deposit_and_stream_status() {
    let (env, _admin, hardware_id, user, _contract_id) = setup();

    let client = EnergyGatewayClient::new(&env, &_contract_id);

    client.deposit_stream(&user, &500);

    let status = client.get_stream_status(&hardware_id).unwrap();
    assert!(status);
}

#[test]
fn test_deposit_zero_fails() {
    let (_env, _admin, _hardware_id, user, _contract_id) = setup();

    let client = EnergyGatewayClient::new(&_env, &_contract_id);
    let result = client.try_deposit_stream(&user, &0);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn test_deposit_negative_fails() {
    let (_env, _admin, _hardware_id, user, _contract_id) = setup();

    let client = EnergyGatewayClient::new(&_env, &_contract_id);
    let result = client.try_deposit_stream(&user, &-100);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn test_settle_telemetry_happy_path() {
    let (_env, _admin, hardware_id, user, _contract_id) = setup();

    let client = EnergyGatewayClient::new(&_env, &_contract_id);

    client.deposit_stream(&user, &1000);

    let cost = client.settle_telemetry(&hardware_id, &50).unwrap();
    assert_eq!(cost, 500);

    let status = client.get_stream_status(&hardware_id).unwrap();
    assert!(status);
}

#[test]
fn test_settle_telemetry_exhausts_balance() {
    let (_env, _admin, hardware_id, user, _contract_id) = setup();

    let client = EnergyGatewayClient::new(&_env, &_contract_id);

    client.deposit_stream(&user, &200);

    let cost = client.settle_telemetry(&hardware_id, &20).unwrap();
    assert_eq!(cost, 200);

    let status = client.get_stream_status(&hardware_id).unwrap();
    assert!(!status);
}

#[test]
fn test_insufficient_funds_fails() {
    let (_env, _admin, hardware_id, user, _contract_id) = setup();

    let client = EnergyGatewayClient::new(&_env, &_contract_id);

    client.deposit_stream(&user, &50);

    let result = client.try_settle_telemetry(&hardware_id, &100);
    assert_eq!(result, Err(Ok(Error::InsufficientFunds)));
}

#[test]
fn test_unauthorized_cannot_settle() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let hardware_id = BytesN::from_array(&env, &[0xca; 32]);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin.clone());

    let contract_id = env.register_contract(None, EnergyGateway);
    let client = EnergyGatewayClient::new(&env, &contract_id);

    env.mock_all_auths();
    client.initialize(&admin, &hardware_id, &token_contract, &10);
    env.mock_all_auths();

    let result = client.try_settle_telemetry(&hardware_id, &1);
    assert!(result.is_err());
}

#[test]
fn test_hardware_id_mismatch_fails() {
    let (_env, _admin, _hardware_id, user, _contract_id) = setup();

    let wrong_id = BytesN::from_array(&_env, &[0x00; 32]);
    let client = EnergyGatewayClient::new(&_env, &_contract_id);

    client.deposit_stream(&user, &1000);

    let result = client.try_settle_telemetry(&wrong_id, &10);
    assert_eq!(result, Err(Ok(Error::HardwareIdMismatch)));
}

#[test]
fn test_not_initialized_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let hardware_id = BytesN::from_array(&env, &[0xca; 32]);

    let contract_id = env.register_contract(None, EnergyGateway);
    let client = EnergyGatewayClient::new(&env, &contract_id);

    let result = client.try_get_stream_status(&hardware_id);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

#[test]
fn test_multiple_users_and_deposits() {
    let (_env, admin, hardware_id, user_a, _contract_id) = setup();
    let user_b = Address::generate(&_env);

    let token_addr: Address = _env
        .storage()
        .instance()
        .get(&DataKey::TokenAddress)
        .unwrap();
    let sac = token::StellarAssetClient::new(&_env, &token_addr);
    sac.mint(&user_b, &1_000_000);

    let token_client = token::Client::new(&_env, &token_addr);
    token_client.approve(&user_b, &_contract_id, &1_000_000, &u32::MAX);

    let client = EnergyGatewayClient::new(&_env, &_contract_id);

    client.deposit_stream(&user_a, &1000);
    client.deposit_stream(&user_b, &2000);

    let cost = client.settle_telemetry(&hardware_id, &15).unwrap();
    assert_eq!(cost, 150);

    let status = client.get_stream_status(&hardware_id).unwrap();
    assert!(status);

    let cost = client.settle_telemetry(&hardware_id, &285).unwrap();
    assert_eq!(cost, 2850);

    let status = client.get_stream_status(&hardware_id).unwrap();
    assert!(!status);
}

#[test]
fn test_events_emitted() {
    let (_env, _admin, hardware_id, user, _contract_id) = setup();

    let client = EnergyGatewayClient::new(&_env, &_contract_id);
    client.deposit_stream(&user, &1000);
    client.settle_telemetry(&hardware_id, &30).unwrap();

    let events = _env.events().all();
    let has_sfund = events.iter().any(|e| e.topics.first() == Some(&STREAM_FUNDED));
    let has_tstl = events.iter().any(|e| e.topics.first() == Some(&TELEMETRY_SETTLED));
    assert!(has_sfund, "expected STREAM_FUNDED event");
    assert!(has_tstl, "expected TELEMETRY_SETTLED event");
}

#[test]
fn test_relay_triggered_event_on_dry() {
    let (_env, _admin, hardware_id, user, _contract_id) = setup();

    let client = EnergyGatewayClient::new(&_env, &_contract_id);
    client.deposit_stream(&user, &100);
    client.settle_telemetry(&hardware_id, &10).unwrap();

    let events = _env.events().all();
    let has_rely = events.iter().any(|e| e.topics.first() == Some(&RELAY_TRIGGERED));
    assert!(has_rely, "expected RELAY_TRIGGERED event");
}
