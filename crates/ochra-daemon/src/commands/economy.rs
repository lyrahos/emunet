//! Economy & Oracle command handlers (Section 21.3).

use std::sync::Arc;

use serde_json::Value;

use crate::rpc::RpcError;
use crate::DaemonState;

type Result = std::result::Result<Value, RpcError>;

/// Get Oracle TWAP and circuit breaker status.
pub async fn get_oracle_twap(_state: &Arc<DaemonState>) -> Result {
    // v1: Hardcoded oracle rate (1 Seed = 1 USD = 100_000_000 micro-seeds)
    Ok(serde_json::json!({
        "seed_value": 100_000_000_u64,
        "is_circuit_breaker_active": false,
        "stale_hours": 0,
    }))
}

/// Get wallet balance.
pub async fn get_wallet_balance(state: &Arc<DaemonState>) -> Result {
    let db = state.db.lock().await;
    let balance = ochra_db::queries::wallet::balance(&db)
        .map_err(|e| RpcError::internal_error(&format!("db error: {e}")))?;

    Ok(serde_json::json!({
        "stable_seeds": balance,
        "yield_shares": 0_u64,
        "yield_decay_rate": 0.0_f32,
    }))
}

/// Get purchase history.
pub async fn get_purchase_history(state: &Arc<DaemonState>) -> Result {
    let db = state.db.lock().await;
    let txs = ochra_db::queries::wallet::recent_transactions(&db, 100)
        .map_err(|e| RpcError::internal_error(&format!("db error: {e}")))?;

    let result: Vec<Value> = txs
        .iter()
        .map(|tx| {
            serde_json::json!({
                "tx_hash": hex::encode(&tx.tx_hash),
                "tx_type": tx.tx_type,
                "amount": tx.amount,
                "epoch": tx.epoch,
                "timestamp": tx.timestamp,
            })
        })
        .collect();

    Ok(serde_json::json!(result))
}

/// Send funds to a recipient.
pub async fn send_funds(state: &Arc<DaemonState>, params: &Value) -> Result {
    let _recipient_pik = params
        .get("recipient_pik")
        .ok_or_else(|| RpcError::invalid_params("recipient_pik required"))?;
    let amount = params
        .get("amount_seeds")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| RpcError::invalid_params("amount_seeds required"))?;

    // Check balance
    let db = state.db.lock().await;
    let balance = ochra_db::queries::wallet::balance(&db)
        .map_err(|e| RpcError::internal_error(&format!("db error: {e}")))?;

    if balance < amount {
        return Err(RpcError::insufficient_balance(amount, balance));
    }

    // Would create transaction, update wallet, gossip nullifier
    let tx_hash = ochra_crypto::blake3::hash(&amount.to_le_bytes());

    Ok(serde_json::json!({
        "tx_hash": hex::encode(tx_hash),
    }))
}

/// Force flush service receipts for immediate minting.
pub async fn force_flush_receipts(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _proof = params
        .get("groth16_proof")
        .ok_or_else(|| RpcError::invalid_params("groth16_proof required"))?;

    Ok(serde_json::json!({
        "receipts_flushed": 0,
        "seeds_minted": 0,
    }))
}

/// Initialize a TLS notary share (Oracle MPC).
pub async fn init_tls_notary_share(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _target_api = params
        .get("target_api")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("target_api required"))?;

    Ok(serde_json::json!({
        "session_id": "stub-mpc-session",
        "status": "initialized",
    }))
}

/// Propose a revenue split change (30-day timelock).
pub async fn propose_revenue_split(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _group_id = params
        .get("group_id")
        .ok_or_else(|| RpcError::invalid_params("group_id required"))?;
    let _new_split = params
        .get("new_split")
        .ok_or_else(|| RpcError::invalid_params("new_split required"))?;

    Ok(serde_json::json!({
        "status": "pending",
        "effective_at": 0,
    }))
}

/// Get earnings breakdown for a Space.
pub async fn get_earnings_breakdown(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _group_id = params
        .get("group_id")
        .ok_or_else(|| RpcError::invalid_params("group_id required"))?;

    Ok(serde_json::json!({
        "total_earned": 0_u64,
        "host_earned": 0_u64,
        "creator_earned": 0_u64,
        "network_earned": 0_u64,
        "epoch": 0,
    }))
}

/// Claim VYS rewards.
pub async fn claim_vys_rewards(_state: &Arc<DaemonState>) -> Result {
    Ok(serde_json::json!({
        "amount": 0_u64,
        "epoch": crate::epoch::current_epoch(),
    }))
}

/// Request an anonymous refund.
pub async fn request_anonymous_refund(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _content_hash = params
        .get("content_hash")
        .ok_or_else(|| RpcError::invalid_params("content_hash required"))?;
    let _tier_index = params
        .get("tier_index")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| RpcError::invalid_params("tier_index required"))?;

    Ok(serde_json::json!({
        "status": "pending",
        "refund_amount": 0,
    }))
}

/// Get current collateral ratio.
pub async fn get_collateral_ratio(_state: &Arc<DaemonState>) -> Result {
    Ok(serde_json::json!({
        "current_cr": 1.0_f32,
        "trend": "stable",
    }))
}

/// Get circulating supply.
pub async fn get_circulating_supply(_state: &Arc<DaemonState>) -> Result {
    Ok(serde_json::json!(0_u64))
}

/// Dev-only: Set oracle rate for testing.
pub async fn dev_set_oracle_rate(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _rate = params
        .get("rate")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| RpcError::invalid_params("rate required"))?;

    Ok(serde_json::json!({"rate_set": true}))
}
