use std::io::{Cursor, Read, Write};

use anyhow::Context;
use ark_serialize::CanonicalDeserialize;
use decaf377::Fq;
use f4jumble::{f4jumble, f4jumble_inv};
use penumbra_proto::{penumbra::core::keys::v1 as pb, serializers::bech32str, DomainType};
use rand::{CryptoRng, Rng};
use serde::{Deserialize, Serialize};

mod r1cs;
pub use r1cs::AddressVar;

mod view;
pub use view::AddressView;

use crate::{fmd, ka, keys::Diversifier};

pub const ADDRESS_LEN_BYTES: usize = 80;
/// Number of bits in the address short form divided by the number of bits per Bech32m character
pub const ADDRESS_NUM_CHARS_SHORT_FORM: usize = 24;

/// A valid payment address.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "pb::Address", into = "pb::Address")]
pub struct Address {
    d: Diversifier,
    /// cached copy of the diversified base
    g_d: decaf377::Element,

    /// extra invariant: the bytes in pk_d should be the canonical encoding of an
    /// s value (whether or not it is a valid decaf377 encoding)
    /// this ensures we can use a PaymentAddress to form a note commitment,
    /// which involves hashing s as a field element.
    pk_d: ka::Public,
    /// transmission key s value
    transmission_key_s: Fq,

    ck_d: fmd::ClueKey,
}

impl std::cmp::PartialOrd for Address {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.to_vec().cmp(&other.to_vec()))
    }
}

impl std::cmp::Ord for Address {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_vec().cmp(&other.to_vec())
    }
}

impl std::hash::Hash for Address {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_vec().hash(state)
    }
}

impl Address {
    /// Constructs a payment address from its components.
    ///
    /// Returns `None` if the bytes in pk_d are a non-canonical representation
    /// of an [`Fq`] `s` value.
    pub fn from_components(d: Diversifier, pk_d: ka::Public, ck_d: fmd::ClueKey) -> Option<Self> {
        // XXX ugly -- better way to get our hands on the s value?
        // add to decaf377::Encoding? there's compress_to_field already...
        if let Ok(transmission_key_s) = Fq::deserialize_compressed(&pk_d.0[..]) {
            // don't need an error type here, caller will probably .expect anyways
            Some(Self {
                d,
                g_d: d.diversified_generator(),
                pk_d,
                ck_d,
                transmission_key_s,
            })
        } else {
            None
        }
    }

    pub fn diversifier(&self) -> &Diversifier {
        &self.d
    }

    pub fn diversified_generator(&self) -> &decaf377::Element {
        &self.g_d
    }

    pub fn transmission_key(&self) -> &ka::Public {
        &self.pk_d
    }

    pub fn clue_key(&self) -> &fmd::ClueKey {
        &self.ck_d
    }

    pub fn transmission_key_s(&self) -> &Fq {
        &self.transmission_key_s
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        bytes
            .write_all(&self.diversifier().0)
            .expect("can write diversifier into vec");
        bytes
            .write_all(&self.transmission_key().0)
            .expect("can write transmission key into vec");
        bytes
            .write_all(&self.clue_key().0)
            .expect("can write clue key into vec");

        f4jumble(bytes.get_ref()).expect("can jumble")
    }

    /// A randomized dummy address.
    pub fn dummy<R: CryptoRng + Rng>(rng: &mut R) -> Self {
        loop {
            let mut diversifier_bytes = [0u8; 16];
            rng.fill_bytes(&mut diversifier_bytes);

            let mut pk_d_bytes = [0u8; 32];
            rng.fill_bytes(&mut pk_d_bytes);

            let mut clue_key_bytes = [0; 32];
            rng.fill_bytes(&mut clue_key_bytes);

            let diversifier = Diversifier(diversifier_bytes);
            let addr = Address::from_components(
                diversifier,
                ka::Public(pk_d_bytes),
                fmd::ClueKey(clue_key_bytes),
            );

            if let Some(addr) = addr {
                return addr;
            }
        }
    }

    /// Short form suitable for displaying in a UI.
    pub fn display_short_form(&self) -> String {
        let full_address = format!("{self}");
        // Fixed prefix is `penumbrav2t` plus the Bech32m separator `1`.
        let fixed_prefix = format!("{}{}", bech32str::address::BECH32_PREFIX, '1');
        let num_chars_to_display = fixed_prefix.len() + ADDRESS_NUM_CHARS_SHORT_FORM;

        format!("{}…", &full_address[0..num_chars_to_display])
    }

    /// Compat (bech32 non-m) address format
    pub fn compat_encoding(&self) -> String {
        let proto_address = pb::Address::from(*self);
        bech32str::encode(
            &proto_address.inner,
            bech32str::compat_address::BECH32_PREFIX,
            bech32str::Bech32,
        )
    }
}

impl DomainType for Address {
    type Proto = pb::Address;
}

impl From<Address> for pb::Address {
    fn from(a: Address) -> Self {
        pb::Address {
            inner: a.to_vec(),
            // Always produce encodings without the alt format.
            alt_bech32m: String::new(),
        }
    }
}

impl TryFrom<pb::Address> for Address {
    type Error = anyhow::Error;

    fn try_from(value: pb::Address) -> Result<Self, Self::Error> {
        match (value.inner.is_empty(), value.alt_bech32m.is_empty()) {
            (false, true) => value.inner.try_into(),
            (true, false) => value.alt_bech32m.parse(),
            (false, false) => Err(anyhow::anyhow!(
                "Address proto has both inner and alt_bech32m fields set"
            )),
            (true, true) => Err(anyhow::anyhow!(
                "Address proto has neither inner nor alt_bech32m fields set"
            )),
        }
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let proto_address = pb::Address::from(*self);
        f.write_str(&bech32str::encode(
            &proto_address.inner,
            bech32str::address::BECH32_PREFIX,
            bech32str::Bech32m,
        ))
    }
}

impl std::fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        <Self as std::fmt::Display>::fmt(self, f)
    }
}

impl std::str::FromStr for Address {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with(bech32str::compat_address::BECH32_PREFIX) {
            pb::Address {
                inner: bech32str::decode(
                    s,
                    bech32str::compat_address::BECH32_PREFIX,
                    bech32str::Bech32,
                )?,
                alt_bech32m: String::new(),
            }
            .try_into()
        } else {
            pb::Address {
                inner: bech32str::decode(s, bech32str::address::BECH32_PREFIX, bech32str::Bech32m)?,
                alt_bech32m: String::new(),
            }
            .try_into()
        }
    }
}

impl TryFrom<Vec<u8>> for Address {
    type Error = anyhow::Error;

    fn try_from(jumbled_vec: Vec<u8>) -> Result<Self, Self::Error> {
        (&jumbled_vec[..]).try_into()
    }
}

impl TryFrom<&Vec<u8>> for Address {
    type Error = anyhow::Error;

    fn try_from(jumbled_vec: &Vec<u8>) -> Result<Self, Self::Error> {
        (jumbled_vec[..]).try_into()
    }
}

impl TryFrom<&[u8]> for Address {
    type Error = anyhow::Error;

    fn try_from(jumbled_bytes: &[u8]) -> Result<Self, Self::Error> {
        if jumbled_bytes.len() != ADDRESS_LEN_BYTES {
            anyhow::bail!("address malformed");
        }

        let unjumbled_bytes =
            f4jumble_inv(jumbled_bytes).ok_or_else(|| anyhow::anyhow!("invalid address"))?;
        let mut bytes = Cursor::new(unjumbled_bytes);

        let mut diversifier_bytes = [0u8; 16];
        bytes
            .read_exact(&mut diversifier_bytes)
            .context("could not read diversifier bytes")?;

        let mut pk_d_bytes = [0u8; 32];
        bytes
            .read_exact(&mut pk_d_bytes)
            .context("could not read transmission key bytes")?;

        let mut clue_key_bytes = [0; 32];
        bytes
            .read_exact(&mut clue_key_bytes)
            .context("could not read clue key bytes")?;

        let diversifier = Diversifier(diversifier_bytes);

        Address::from_components(
            diversifier,
            ka::Public(pk_d_bytes),
            fmd::ClueKey(clue_key_bytes),
        )
        .ok_or_else(|| anyhow::anyhow!("could not create address from components"))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rand_core::OsRng;

    use super::*;
    use crate::keys::{Bip44Path, SeedPhrase, SpendKey};

    #[test]
    fn test_address_encoding() {
        let rng = OsRng;
        let seed_phrase = SeedPhrase::generate(rng);
        let sk = SpendKey::from_seed_phrase_bip44(seed_phrase, &Bip44Path::new(0));
        let fvk = sk.full_viewing_key();
        let ivk = fvk.incoming();
        let (dest, _dtk_d) = ivk.payment_address(0u32.into());

        let bech32m_addr = format!("{dest}");

        let addr = Address::from_str(&bech32m_addr).expect("can decode valid address");

        use penumbra_proto::Message;

        let proto_addr = dest.encode_to_vec();
        let proto_addr_bech32m = pb::Address {
            inner: Vec::new(),
            alt_bech32m: bech32m_addr,
        }
        .encode_to_vec();
        let proto_addr_direct: pb::Address = dest.into();
        let addr_from_proto: Address = proto_addr_direct
            .try_into()
            .expect("can convert from proto back to address");

        let addr2 = Address::decode(proto_addr.as_ref()).expect("can decode valid address");
        let addr3 = Address::decode(proto_addr_bech32m.as_ref()).expect("can decode valid address");

        assert_eq!(addr, dest);
        assert_eq!(addr2, dest);
        assert_eq!(addr3, dest);
        assert_eq!(addr_from_proto, dest);
    }

    #[test]
    fn test_bytes_roundtrip() {
        let rng = OsRng;
        let seed_phrase = SeedPhrase::generate(rng);
        let sk = SpendKey::from_seed_phrase_bip44(seed_phrase, &Bip44Path::new(0));
        let fvk = sk.full_viewing_key();
        let ivk = fvk.incoming();
        let (dest, _dtk_d) = ivk.payment_address(0u32.into());

        let bytes = dest.to_vec();
        let addr: Address = bytes.try_into().expect("can decode valid address");

        assert_eq!(addr, dest);
    }

    #[test]
    fn test_address_keys_are_diversified() {
        let rng = OsRng;
        let seed_phrase = SeedPhrase::generate(rng);
        let sk = SpendKey::from_seed_phrase_bip44(seed_phrase, &Bip44Path::new(0));
        let fvk = sk.full_viewing_key();
        let ivk = fvk.incoming();
        let (dest1, dtk_d1) = ivk.payment_address(0u32.into());
        let (dest2, dtk_d2) = ivk.payment_address(1u32.into());

        assert!(dest1.transmission_key() != dest2.transmission_key());
        assert!(dest1.clue_key() != dest2.clue_key());
        assert!(dtk_d1.to_bytes() != dtk_d2.to_bytes());
    }
}
