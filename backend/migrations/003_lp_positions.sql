CREATE TABLE IF NOT EXISTS lp_positions (
    id UUID PRIMARY KEY,
    wallet_address VARCHAR(255) NOT NULL,
    protocol VARCHAR(50) NOT NULL CHECK (protocol IN ('AgniFinance', 'MerchantMoe')),

    -- Agni-specific fields
    agni_token_id BIGINT,
    agni_token0 VARCHAR(255),
    agni_token1 VARCHAR(255),
    agni_fee INTEGER,
    agni_tick_lower INTEGER,
    agni_tick_upper INTEGER,
    agni_liquidity NUMERIC,

    -- Merchant Moe-specific fields
    moe_lb_pair VARCHAR(255),
    moe_token_x VARCHAR(255),
    moe_token_y VARCHAR(255),
    moe_bin_step INTEGER,
    moe_bin_ids BIGINT[],
    moe_liquidity_minted NUMERIC[],

    -- Common fields
    amount_x_added NUMERIC NOT NULL,
    amount_y_added NUMERIC NOT NULL,
    intent_hash VARCHAR(255),
    tx_hash VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    -- Indexes for efficient querying
    FOREIGN KEY (wallet_address) REFERENCES users(wallet_address) ON DELETE CASCADE
);

CREATE INDEX idx_lp_positions_wallet ON lp_positions(wallet_address);
CREATE INDEX idx_lp_positions_protocol ON lp_positions(protocol);
CREATE INDEX idx_lp_positions_tx_hash ON lp_positions(tx_hash);
CREATE INDEX idx_lp_positions_intent_hash ON lp_positions(intent_hash);
