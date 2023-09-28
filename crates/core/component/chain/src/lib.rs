#![deny(clippy::unwrap_used)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod epoch;
mod known_assets;
mod note_source;

#[cfg_attr(docsrs, doc(cfg(feature = "component")))]
#[cfg(feature = "component")]
pub mod component;

pub mod genesis;
pub mod params;
pub mod state_key;

pub use epoch::Epoch;
pub use known_assets::KnownAssets;
pub use note_source::{NoteSource, SpendInfo};

/// Hardcoded test data used by the `Default` genesis state.
pub mod test_keys {
    use once_cell::sync::Lazy;
    use penumbra_keys::{
        keys::{AccountGroupId, SpendKey},
        Address, FullViewingKey,
    };

    /// This address is for test purposes, allocations were added beginning with
    /// the 016-Pandia testnet.
    pub const SEED_PHRASE: &str = "benefit cherry cannon tooth exhibit law avocado spare tooth that amount pumpkin scene foil tape mobile shine apology add crouch situate sun business explain";

    /// These addresses both correspond to the test wallet above.
    pub const ADDRESS_0_STR: &str = "penumbrav2t13vh0fkf3qkqjacpm59g23ufea9n5us45e4p5h6hty8vg73r2t8g5l3kynad87u0n9eragf3hhkgkhqe5vhngq2cw493k48c9qg9ms4epllcmndd6ly4v4dw2jcnxaxzjqnlvnw";
    /// These addresses both correspond to the test wallet above.
    pub const ADDRESS_1_STR: &str = "penumbrav2t1gl609fq6xzjcqn3hz3crysw2s0nkt330lyhaq403ztmrm3yygsgdklt9uxfs0gedwp6sypp5k5ln9t62lvs9t0a990q832wnxak8r939g5u6uz5aessd8saxvv7ewlz4hhqnws";

    pub static ADDRESS_0: Lazy<Address> = Lazy::new(|| {
        ADDRESS_0_STR
            .parse()
            .expect("hardcoded test addresses should be valid")
    });
    pub static ADDRESS_1: Lazy<Address> = Lazy::new(|| {
        ADDRESS_1_STR
            .parse()
            .expect("hardcoded test addresses should be valid")
    });

    pub static SPEND_KEY: Lazy<SpendKey> = Lazy::new(|| {
        SpendKey::from_seed_phrase_bip39(
            SEED_PHRASE
                .parse()
                .expect("hardcoded test seed phrase should be valid"),
            0,
        )
    });

    pub static FULL_VIEWING_KEY: Lazy<FullViewingKey> =
        Lazy::new(|| SPEND_KEY.full_viewing_key().clone());

    pub static ACCOUNT_ID: Lazy<AccountGroupId> = Lazy::new(|| FULL_VIEWING_KEY.account_group_id());
}

// Located here at the bottom of the dep tree for convenience
mod effect_hash;
mod transaction;
pub use effect_hash::{EffectHash, EffectingData};
pub use transaction::TransactionContext;
