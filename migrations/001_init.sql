CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  wallet_address TEXT UNIQUE NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_seen_at TIMESTAMPTZ
);

CREATE TABLE provider_snapshots (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  provider TEXT NOT NULL,
  data_type TEXT NOT NULL,
  entity_key TEXT NOT NULL,
  source_event_id TEXT,
  payload JSONB NOT NULL,
  captured_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE(provider, data_type, entity_key, source_event_id)
);

CREATE TABLE signals (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  category TEXT NOT NULL CHECK (category IN ('ALPHA', 'ANOMALY', 'RISK', 'OPPORTUNITY')),
  headline TEXT NOT NULL,
  explanation TEXT NOT NULL,
  confidence_score INTEGER NOT NULL CHECK (confidence_score BETWEEN 0 AND 100),
  related_wallet TEXT,
  related_protocol TEXT,
  related_asset TEXT,
  source_provider TEXT NOT NULL,
  source_data JSONB NOT NULL,
  input_facts_hash TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE agent_intents (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  wallet_address TEXT NOT NULL,
  raw_intent TEXT NOT NULL,
  parsed_intent JSONB NOT NULL,
  execution_mode TEXT NOT NULL CHECK (execution_mode IN ('INSTANT', 'RECURRING', 'CONDITIONAL', 'RECURRING_CONDITIONAL')),
  status TEXT NOT NULL CHECK (status IN ('DRAFT', 'ACTIVE', 'PAUSED', 'COMPLETED', 'CANCELLED')),
  intent_hash TEXT NOT NULL UNIQUE,
  onchain_intent_id BIGINT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE agent_reasoning_logs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  intent_id UUID REFERENCES agent_intents(id),
  action_type TEXT NOT NULL,
  explanation TEXT NOT NULL,
  confidence_score INTEGER CHECK (confidence_score BETWEEN 0 AND 100),
  reasoning_hash TEXT,
  tx_hash TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE agent_execution_policies (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  intent_id UUID REFERENCES agent_intents(id),
  wallet_address TEXT NOT NULL,
  allowed_assets JSONB NOT NULL,
  allowed_protocols JSONB NOT NULL,
  max_spend_usd NUMERIC,
  max_transaction_count INTEGER,
  expires_at TIMESTAMPTZ NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('DRAFT', 'ACTIVE', 'PAUSED', 'COMPLETED', 'CANCELLED')),
  policy_hash TEXT NOT NULL UNIQUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE agent_execution_logs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  intent_id UUID REFERENCES agent_intents(id),
  policy_id UUID REFERENCES agent_execution_policies(id),
  action_type TEXT NOT NULL,
  proposed_action JSONB NOT NULL,
  execution_status TEXT NOT NULL,
  tx_hash TEXT,
  reasoning_hash TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE portfolio_identities (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  wallet_address TEXT UNIQUE NOT NULL,
  archetype TEXT NOT NULL,
  percentile INTEGER CHECK (percentile BETWEEN 0 AND 100),
  stats JSONB NOT NULL,
  insights JSONB NOT NULL,
  metadata_uri TEXT,
  sbt_token_id BIGINT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE arena_predictions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  onchain_prediction_id BIGINT,
  claim TEXT NOT NULL,
  metric TEXT NOT NULL,
  target_value NUMERIC NOT NULL,
  comparison_operator TEXT NOT NULL CHECK (comparison_operator IN ('GTE', 'LTE')),
  expiry_time TIMESTAMPTZ NOT NULL,
  seer_position TEXT NOT NULL CHECK (seer_position IN ('BACK_SEER', 'CHALLENGE_SEER')),
  seer_confidence INTEGER NOT NULL CHECK (seer_confidence BETWEEN 0 AND 100),
  reasoning TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('OPEN', 'LOCKED', 'RESOLVED', 'CANCELLED')),
  result TEXT CHECK (result IN ('SEER_CORRECT', 'SEER_INCORRECT', 'VOID')),
  final_value NUMERIC,
  input_facts_hash TEXT,
  tx_hash TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  resolved_at TIMESTAMPTZ
);

CREATE TABLE arena_entries (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  prediction_id UUID REFERENCES arena_predictions(id),
  wallet_address TEXT NOT NULL,
  user_position TEXT NOT NULL CHECK (user_position IN ('BACK_SEER', 'CHALLENGE_SEER')),
  points_committed INTEGER NOT NULL CHECK (points_committed > 0),
  status TEXT NOT NULL CHECK (status IN ('ACTIVE', 'RESOLVED', 'CANCELLED')),
  points_delta INTEGER,
  tx_hash TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  resolved_at TIMESTAMPTZ
);

CREATE TABLE arena_leaderboard_cache (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  wallet_address TEXT NOT NULL,
  total_points INTEGER NOT NULL,
  weekly_gain INTEGER NOT NULL,
  accuracy_rate NUMERIC,
  entries_count INTEGER NOT NULL,
  rank INTEGER NOT NULL,
  calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_signals_created_at ON signals(created_at DESC);
CREATE INDEX idx_agent_intents_wallet ON agent_intents(wallet_address);
CREATE INDEX idx_arena_predictions_status_expiry ON arena_predictions(status, expiry_time);
CREATE INDEX idx_arena_entries_wallet ON arena_entries(wallet_address);
