#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracterror, contracttype, symbol_short, token, Address, BytesN, Env,
    Map, Symbol,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    Unauthorized = 2,
    InsufficientFunds = 3,
    NotInitialized = 4,
    HardwareIdMismatch = 5,
    InvalidAmount = 6,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    HardwareId,
    TokenAddress,
    RatePerWh,
    IsInitialized,
    Balances,
    TotalBalance,
}

const STREAM_FUNDED: Symbol = symbol_short!("SFUND");
const TELEMETRY_SETTLED: Symbol = symbol_short!("TSTL");
const RELAY_TRIGGERED: Symbol = symbol_short!("RELY");

#[contract]
pub struct EnergyGateway;

#[contractimpl]
impl EnergyGateway {
    pub fn initialize(
        env: Env,
        admin: Address,
        hardware_id: BytesN<32>,
        token: Address,
        rate_per_wh: u64,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::IsInitialized) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::HardwareId, &hardware_id);
        env.storage()
            .instance()
            .set(&DataKey::TokenAddress, &token);
        env.storage()
            .instance()
            .set(&DataKey::RatePerWh, &rate_per_wh);
        env.storage()
            .instance()
            .set(&DataKey::TotalBalance, &0i128);
        env.storage()
            .instance()
            .set(&DataKey::IsInitialized, &true);
        Ok(())
    }

    pub fn deposit_stream(env: Env, user: Address, amount: i128) -> Result<(), Error> {
        Self::check_initialized(&env)?;
        user.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let token: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenAddress)
            .unwrap();
        let contract_id = env.current_contract_address();

        token::Client::new(&env, &token).transfer_from(
            &contract_id,
            &user,
            &contract_id,
            &amount,
        );

        let mut balances: Map<Address, i128> = env
            .storage()
            .instance()
            .get(&DataKey::Balances)
            .unwrap_or(Map::new(&env));
        let balance = balances.get(user.clone()).unwrap_or(0);
        balances.set(user.clone(), balance + amount);
        env.storage().instance().set(&DataKey::Balances, &balances);

        let total: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalBalance)
            .unwrap();
        env.storage()
            .instance()
            .set(&DataKey::TotalBalance, &(total + amount));

        env.events().publish(
            (STREAM_FUNDED, user, balance + amount),
            amount,
        );

        Ok(())
    }

    pub fn settle_telemetry(
        env: Env,
        hardware_id: BytesN<32>,
        watt_hours_consumed: u64,
    ) -> Result<i128, Error> {
        Self::check_initialized(&env)?;

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let stored_id: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::HardwareId)
            .unwrap();
        if hardware_id != stored_id {
            return Err(Error::HardwareIdMismatch);
        }

        let rate: u64 = env.storage().instance().get(&DataKey::RatePerWh).unwrap();
        let cost: i128 = (watt_hours_consumed as i128) * (rate as i128);
        if cost <= 0 {
            return Err(Error::InvalidAmount);
        }

        let total: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalBalance)
            .unwrap();
        if total < cost {
            return Err(Error::InsufficientFunds);
        }

        Self::deduct_from_balances(&env, cost)?;

        let new_total = total - cost;
        env.storage()
            .instance()
            .set(&DataKey::TotalBalance, &new_total);

        let token: Address = env.storage().instance().get(&DataKey::TokenAddress).unwrap();
        token::Client::new(&env, &token).transfer(
            &env.current_contract_address(),
            &admin,
            &cost,
        );

        let has_funds = new_total > 0;

        env.events().publish(
            (TELEMETRY_SETTLED, hardware_id.clone(), watt_hours_consumed, cost),
            has_funds,
        );

        if !has_funds {
            env.events().publish((RELAY_TRIGGERED, hardware_id), cost);
        }

        Ok(cost)
    }

    pub fn get_stream_status(env: Env, hardware_id: BytesN<32>) -> Result<bool, Error> {
        Self::check_initialized(&env)?;

        let stored_id: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::HardwareId)
            .unwrap();
        if hardware_id != stored_id {
            return Err(Error::HardwareIdMismatch);
        }

        let total: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalBalance)
            .unwrap();
        Ok(total > 0)
    }

    fn check_initialized(env: &Env) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::IsInitialized) {
            Ok(())
        } else {
            Err(Error::NotInitialized)
        }
    }

    fn deduct_from_balances(env: &Env, mut amount: i128) -> Result<(), Error> {
        let balances: Map<Address, i128> = env
            .storage()
            .instance()
            .get(&DataKey::Balances)
            .unwrap();
        let mut updated: Map<Address, i128> = Map::new(env);

        for key in balances.keys() {
            if amount <= 0 {
                updated.set(key.clone(), balances.get(key.clone()).unwrap());
                continue;
            }
            let balance = balances.get(key.clone()).unwrap();
            if balance <= 0 {
                continue;
            }
            let deduct = if balance >= amount { amount } else { balance };
            let new_bal = balance - deduct;
            amount -= deduct;
            if new_bal > 0 {
                updated.set(key.clone(), new_bal);
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::Balances, &updated);

        Ok(())
    }
}

#[cfg(test)]
mod tests;
