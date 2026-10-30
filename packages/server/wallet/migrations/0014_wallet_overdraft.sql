BEGIN;
-- Forward-only: streaming requests may overdraft to the configured lower bound
-- (docs/22 §4 "流式已输出部分可透支到下限"), so the balance columns may go
-- negative for streaming settlement. The application enforces the floor
-- (ZEOKEEL_STREAM_NEGATIVE_FLOOR_MICROS) and non-streaming admission still
-- rejects exhausted wallets. Never edit 0003_wallet.sql; this is the expansion.
ALTER TABLE wallet_accounts DROP CONSTRAINT IF EXISTS wallet_accounts_balance_micros_check;
ALTER TABLE wallet_ledger DROP CONSTRAINT IF EXISTS wallet_ledger_balance_after_check;
COMMENT ON COLUMN wallet_accounts.balance_micros IS
    'Balance derived from wallet_ledger; may be negative only for settled streaming overdraft, bounded by the application floor (docs/22 §4).';
COMMENT ON COLUMN wallet_ledger.balance_after IS
    'Ledger-derived balance after this row; negative only for streaming overdraft (docs/22 §4).';
COMMIT;
