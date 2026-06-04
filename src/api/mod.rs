pub mod agent;
pub mod arena;
pub mod auth;
mod auth_guard;
pub mod contracts;
pub mod health;
pub mod identity;
pub mod signals;
pub mod wallet;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health::health))
        .route("/api/version", get(health::version))
        .route("/api/auth/challenge", post(auth::challenge))
        .route("/api/auth/verify", post(auth::verify))
        .route("/api/wallet/:address/summary", get(wallet::summary))
        .route("/api/wallet/:address/activity", get(wallet::activity))
        .route("/api/wallet/:address/risk", get(wallet::risk))
        .route("/api/signals", get(signals::list))
        .route("/api/signals/:id", get(signals::get))
        .route("/api/identity/:address", get(identity::get))
        .route("/api/identity/:address/generate", post(identity::generate))
        .route(
            "/api/identity/:address/mint-metadata",
            post(identity::mint_metadata),
        )
        .route("/api/arena/predictions", get(arena::predictions))
        .route("/api/arena/predictions/:id", get(arena::prediction))
        .route("/api/arena/predictions/:id/enter", post(arena::enter))
        .route("/api/arena/:address/entries", get(arena::entries))
        .route("/api/arena/leaderboard", get(arena::leaderboard))
        .route("/api/arena/seer-record", get(arena::seer_record))
        .route("/api/arena/resolve-due", post(arena::resolve_due))
        .route("/api/contracts/readiness", get(contracts::readiness))
        .route(
            "/api/contracts/send-raw-transaction",
            post(contracts::send_raw_transaction),
        )
        .route(
            "/api/contracts/send-user-operation",
            post(contracts::send_user_operation),
        )
        .route(
            "/api/contracts/user-operation-receipt",
            post(contracts::user_operation_receipt),
        )
        .route(
            "/api/contracts/erc20-allowance",
            post(contracts::erc20_allowance),
        )
        .route("/api/agent/parse-intent", post(agent::parse_intent))
        .route("/api/agent/evaluate-intent", post(agent::evaluate_intent))
        .route("/api/agent/create-intent", post(agent::create_intent))
        .route("/api/agent/:address/intents", get(agent::intents))
        .route("/api/agent/intent/:intent_id", get(agent::intent))
        .route(
            "/api/agent/intent/:intent_id/reasoning",
            get(agent::reasoning),
        )
        .route("/api/agent/intent/:intent_id/pause", post(agent::pause))
        .route(
            "/api/agent/intent/:intent_id/activate",
            post(agent::activate),
        )
        .route(
            "/api/agent/intent/:intent_id/session-policy",
            post(agent::create_session_policy),
        )
        .route(
            "/api/agent/intent/:intent_id/delegated-execute",
            post(agent::delegated_execute),
        )
        .route(
            "/api/agent/policy/:policy_id/revoke",
            post(agent::revoke_policy),
        )
        .route("/api/agent/intent/:intent_id/stop", post(agent::stop))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
