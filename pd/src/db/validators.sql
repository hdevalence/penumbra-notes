CREATE TABLE IF NOT EXISTS validators (
    tm_pubkey bytea NOT NULL PRIMARY KEY,
    voting_power bigint NOT NULL
)