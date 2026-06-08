CREATE TABLE user_settings (
  wallet_address TEXT PRIMARY KEY,
  settings       JSONB        NOT NULL,
  updated_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);
