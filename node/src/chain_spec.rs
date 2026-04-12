//! # Twill Chain Specification
//!
//! Genesis configuration for the Twill Network.
//!
//! 100% mined. No pre-mine, no ICO, no founder allocation, no dev fund.
//! Every TWL is earned. No authority keys. Permissionless from genesis.

use sc_service::ChainType;
use sp_core::{sr25519, Pair};
use sp_runtime::traits::{IdentifyAccount, Verify};
use twill_primitives::*;
use twill_runtime::WASM_BINARY;

pub type ChainSpec = sc_service::GenericChainSpec;

pub type AccountId =
    <<sp_runtime::MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Derive an AccountId from a string seed (for development/testing)
pub fn get_account_id_from_seed(seed: &str) -> AccountId {
    let pair = sr25519::Pair::from_string(&format!("//{}", seed), None)
        .expect("valid seed");
    AccountId::from(pair.public())
}

/// Development chain — single node, test keys, instant feedback.
/// No authority keys needed — blocks are produced via instant-seal.
pub fn development_config() -> Result<ChainSpec, String> {
    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?,
        None,
    )
    .with_name("Twill Development")
    .with_id("twill_dev")
    .with_chain_type(ChainType::Development)
    .with_properties({
        let mut props = serde_json::Map::new();
        props.insert("tokenSymbol".into(), serde_json::json!("TWL"));
        props.insert("tokenDecimals".into(), serde_json::json!(12));
        props.insert("ss58Format".into(), serde_json::json!(42));
        props.into()
    })
    .with_genesis_config_patch(dev_genesis(vec![
        get_account_id_from_seed("Alice"),
        get_account_id_from_seed("Bob"),
        get_account_id_from_seed("Charlie"),
        get_account_id_from_seed("Dave"),
        get_account_id_from_seed("Eve"),
        get_account_id_from_seed("Ferdie"),
    ]))
    .build())
}

/// Testnet chain — multiple nodes, closer to production.
pub fn testnet_config() -> Result<ChainSpec, String> {
    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?,
        None,
    )
    .with_name("Twill Testnet")
    .with_id("twill_testnet")
    .with_chain_type(ChainType::Local)
    .with_genesis_config_patch(dev_genesis(vec![
        get_account_id_from_seed("Alice"),
        get_account_id_from_seed("Bob"),
        get_account_id_from_seed("Charlie"),
    ]))
    .build())
}

/// Mainnet — production chain.
///
/// No pre-funded accounts. No authority keys. Every TWL is mined.
/// Bootnodes are passed at runtime via --bootnodes CLI flag.
/// Generate the raw spec with: twill build-spec --chain mainnet --raw > mainnet-raw.json
pub fn mainnet_config() -> Result<ChainSpec, String> {
    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?,
        None,
    )
    .with_name("Twill Network")
    .with_id("twill")
    .with_chain_type(ChainType::Live)
    .with_protocol_id("twl")
    .with_properties({
        let mut props = serde_json::Map::new();
        props.insert("tokenSymbol".into(), serde_json::json!("TWL"));
        props.insert("tokenDecimals".into(), serde_json::json!(12));
        props.insert("ss58Format".into(), serde_json::json!(42));
        props.into()
    })
    .with_genesis_config_patch(mainnet_genesis())
    .build())
}

/// Production genesis — no endowed accounts, no pre-mine, nothing.
/// The chain starts empty. All TWL is mined from block 1.
fn mainnet_genesis() -> serde_json::Value {
    serde_json::json!({
        "balances": {
            "balances": []
        },
    })
}

/// Genesis config patch.
///
/// Dev accounts get 10,000 TWL each for testing only.
/// In production, the endowed_accounts list is empty — all TWL is mined.
fn dev_genesis(endowed_accounts: Vec<AccountId>) -> serde_json::Value {
    serde_json::json!({
        "balances": {
            "balances": endowed_accounts
                .iter()
                .map(|k| (k.clone(), 10_000 * TWILL))
                .collect::<Vec<_>>(),
        },
    })
}

/// Validate that genesis config is correct
pub fn validate_genesis() -> bool {
    MINING_POOL == TOTAL_SUPPLY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_validates() {
        assert!(validate_genesis());
    }

    #[test]
    fn mining_pool_is_50m() {
        assert_eq!(MINING_POOL, 50_000_000 * TWILL);
    }

    #[test]
    fn total_is_50m() {
        assert_eq!(TOTAL_SUPPLY, 50_000_000 * TWILL);
    }
}
