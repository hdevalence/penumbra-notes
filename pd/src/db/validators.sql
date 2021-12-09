CREATE TABLE IF NOT EXISTS validators (
    tm_pubkey bytea NOT NULL PRIMARY KEY,
    voting_power bigint NOT NULL,
    commission_address varchar NOT NULL,
    commission_rate bigint NOT NULL,
    unclaimed_reward bytea NOT NULL
)