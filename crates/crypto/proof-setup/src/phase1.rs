use ark_ec::Group;
use ark_ff::{One, UniformRand, Zero};
use rand_core::CryptoRngCore;

use crate::dlog;
use crate::group::{pairing, GroupHasher, Hash, F, G1, G2};

/// Check that a given degree is high enough.
///
/// (We use this enough times to warrant separating it out).
const fn is_degree_large_enough(d: usize) -> bool {
    // We need to have at least the index 1 for our x_i values, so we need d >= 2.
    d >= 2
}

// Some utility functions for encoding the relation between CRS length and degree

const fn short_len(d: usize) -> usize {
    (d - 1) + 1
}

const fn short_len_to_degree(l: usize) -> usize {
    l
}

const fn long_len(d: usize) -> usize {
    (2 * d - 2) + 1
}

/// Raw CRS elements, not yet validated for consistency.
#[derive(Clone, Debug)]
pub struct RawCRSElements {
    pub alpha_1: G1,
    pub beta_1: G1,
    pub beta_2: G2,
    pub x_1: Vec<G1>,
    pub x_2: Vec<G2>,
    pub alpha_x_1: Vec<G1>,
    pub beta_x_1: Vec<G1>,
}

impl RawCRSElements {
    /// Extract a degree, if possible, from these elements.
    ///
    /// This can fail if the elements aren't using a consistent degree size,
    /// or this degree isn't large enough.
    fn get_degree(&self) -> Option<usize> {
        let l = self.x_2.len();
        let d = short_len_to_degree(l);
        if !is_degree_large_enough(d) {
            return None;
        }
        if self.alpha_x_1.len() != short_len(d) {
            return None;
        }
        if self.beta_x_1.len() != short_len(d) {
            return None;
        }
        if self.x_1.len() != long_len(d) {
            return None;
        }
        return Some(d);
    }

    /// Validate the internal consistency of these elements, producing a validated struct.
    ///
    /// This checks if the structure of the elements uses the secret scalars
    /// hidden behind the group elements correctly.
    #[must_use]
    pub fn validate(self) -> Option<CRSElements> {
        // 0. Check that we can extract a valid degree out of these elements.
        let d = self.get_degree()?;
        // 1. Check that the elements committing to the secret values are not 0.
        if self.alpha_1.is_zero()
            || self.beta_1.is_zero()
            || self.beta_2.is_zero()
            || self.x_1[1].is_zero()
            || self.x_2[1].is_zero()
        {
            return None;
        }
        // 2. Check that the two beta commitments match.
        if pairing(self.beta_1, G2::generator()) != pairing(G1::generator(), self.beta_2) {
            return None;
        }
        // 3. Check that the x values match on both groups.
        // Todo: use a batched pairing check for this
        if !self
            .x_1
            .iter()
            .zip(self.x_2.iter())
            .all(|(l, r)| pairing(l, G2::generator()) == pairing(G1::generator(), r))
        {
            return None;
        }
        // 4. Check that alpha and x are connected in alpha_x.
        if !self
            .x_2
            .iter()
            .zip(self.alpha_x_1.iter())
            .all(|(x_i, alpha_x_i)| {
                pairing(self.alpha_1, x_i) == pairing(alpha_x_i, G2::generator())
            })
        {
            return None;
        }
        // 5. Check that beta and x are connected in beta_x.
        if !self
            .x_2
            .iter()
            .zip(self.beta_x_1.iter())
            .all(|(x_i, beta_x_i)| pairing(self.beta_1, x_i) == pairing(beta_x_i, G2::generator()))
        {
            return None;
        }
        // 6. Check that the x_i are the correct powers of x.
        if !self
            .x_1
            .iter()
            .zip(self.x_1.iter().skip(1))
            .all(|(x_i, x_i_plus_1)| {
                pairing(x_i, self.x_2[1]) == pairing(x_i_plus_1, G2::generator())
            })
        {
            return None;
        }

        Some(CRSElements {
            degree: d,
            raw: self,
        })
    }

    /// Hash these elements, producing a succinct digest.
    pub fn hash(&self) -> Hash {
        let mut hasher = GroupHasher::new(b"PC$:crs_elmnts");
        hasher.eat_g1(&self.alpha_1);
        hasher.eat_g1(&self.beta_1);
        hasher.eat_g2(&self.beta_2);

        hasher.eat_usize(self.x_1.len());
        for v in &self.x_1 {
            hasher.eat_g1(v);
        }

        hasher.eat_usize(self.x_2.len());
        for v in &self.x_2 {
            hasher.eat_g2(v);
        }

        hasher.eat_usize(self.alpha_x_1.len());
        for v in &self.alpha_x_1 {
            hasher.eat_g1(v);
        }

        hasher.eat_usize(self.beta_x_1.len());
        for v in &self.beta_x_1 {
            hasher.eat_g1(v);
        }

        hasher.finalize_bytes()
    }
}

/// The CRS elements we produce in phase 1.
///
/// Not all elements of the final CRS are present here.
#[derive(Clone, Debug)]
pub struct CRSElements {
    degree: usize,
    raw: RawCRSElements,
}

impl CRSElements {
    /// Generate a "root" CRS, containing the value 1 for each secret element.
    ///
    /// This takes in the degree "d" associated with the circuit we need
    /// to do a setup for, as per the docs.
    ///
    /// Naturally, these elements shouldn't actually be used as-is, but this
    /// serves as a logical basis for the start of the phase.
    pub fn root(d: usize) -> Self {
        assert!(is_degree_large_enough(d));

        let raw = RawCRSElements {
            alpha_1: G1::generator(),
            beta_1: G1::generator(),
            beta_2: G2::generator(),
            x_1: vec![G1::generator(); (2 * d - 2) + 1],
            x_2: vec![G2::generator(); (d - 1) + 1],
            alpha_x_1: vec![G1::generator(); (d - 1) + 1],
            beta_x_1: vec![G1::generator(); (d - 1) + 1],
        };
        Self { degree: d, raw }
    }

    /// Hash these elements, producing a succinct digest.
    pub fn hash(&self) -> Hash {
        // No need to hash the degree, already implied by the lengths of the elements.
        self.raw.hash()
    }
}

/// A linking proof shows knowledge of the new secret elements linking two sets of CRS elements.
///
/// This pets two cats with one hand:
/// 1. We show that we're actually building off of the previous elements.
/// 2. We show that we know the secret elements we're using, avoiding rogue key chicanery.
#[derive(Clone, Copy, Debug)]
struct LinkingProof {
    alpha_proof: dlog::Proof,
    beta_proof: dlog::Proof,
    x_proof: dlog::Proof,
}

/// The max
pub const CONTRIBUTION_HASH_SIZE: usize = 32;

// Note: Don't need constant time equality because we're hashing public data: contributions.

/// The hash of a contribution, providing a unique string for each contribution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContributionHash(pub [u8; CONTRIBUTION_HASH_SIZE]);

impl AsRef<[u8]> for ContributionHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Represents a contribution to phase1 of the ceremony.
///
/// This contribution is linked to a previous contribution, which it builds upon.
///
/// The contribution includes new elements for the CRS, along with a proof that these elements
/// build upon the claimed parent contribution.
#[derive(Clone, Debug)]
pub struct Contribution {
    pub parent: ContributionHash,
    pub new_elements: CRSElements,
    linking_proof: LinkingProof,
}

impl Contribution {
    fn hash(&self) -> ContributionHash {
        let mut hasher = GroupHasher::new(b"PC$:contribution");
        hasher.eat_bytes(self.parent.as_ref());
        hasher.eat_bytes(self.new_elements.hash().as_ref());
        // Note: we could hide this behind another level of indirection, but contribution
        // already uses the internals of the linking proof anyways, so this doesn't
        // feel egregious to me.
        hasher.eat_bytes(self.linking_proof.alpha_proof.hash().as_ref());
        hasher.eat_bytes(self.linking_proof.beta_proof.hash().as_ref());
        hasher.eat_bytes(self.linking_proof.x_proof.hash().as_ref());
        ContributionHash(hasher.finalize_bytes())
    }

    /// Make a new contribution, over the previous CRS elements.
    ///
    /// We also need a contribution hash, for the parent we're building on,
    /// including those elements and other information, which will then appear
    /// in the resulting contribution we're making.
    pub fn make<R: CryptoRngCore>(
        rng: &mut R,
        parent: ContributionHash,
        old: &CRSElements,
    ) -> Self {
        let alpha = F::rand(rng);
        let beta = F::rand(rng);
        let x = F::rand(rng);

        let mut new = old.clone();

        new.raw.alpha_1 *= alpha;
        new.raw.beta_1 *= beta;
        new.raw.beta_2 *= beta;

        let mut x_i = F::one();
        let mut alpha_x_i = alpha;
        let mut beta_x_i = beta;

        let d = old.degree;
        for i in 0..short_len(d) {
            new.raw.x_1[i] *= x_i;
            new.raw.x_2[i] *= x_i;
            new.raw.alpha_x_1[i] *= alpha_x_i;
            new.raw.beta_x_1[i] *= beta_x_i;

            x_i *= x;
            alpha_x_i *= x;
            beta_x_i *= x;
        }
        for i in short_len(d)..long_len(d) {
            new.raw.x_1[i] *= x_i;

            x_i *= x;
        }

        let alpha_proof = dlog::prove(
            rng,
            b"phase1 alpha proof",
            dlog::Statement {
                result: new.raw.alpha_1,
                base: old.raw.alpha_1,
            },
            dlog::Witness { dlog: alpha },
        );
        let beta_proof = dlog::prove(
            rng,
            b"phase1 beta proof",
            dlog::Statement {
                result: new.raw.beta_1,
                base: old.raw.beta_1,
            },
            dlog::Witness { dlog: beta },
        );
        let x_proof = dlog::prove(
            rng,
            b"phase1 x proof",
            dlog::Statement {
                result: new.raw.x_1[1],
                base: old.raw.x_1[1],
            },
            dlog::Witness { dlog: x },
        );

        Self {
            parent,
            new_elements: new,
            linking_proof: LinkingProof {
                alpha_proof,
                beta_proof,
                x_proof,
            },
        }
    }

    /// Verify that this contribution is linked to a previous list of elements.
    #[must_use]
    pub fn is_linked_to(&self, parent: &CRSElements) -> bool {
        // 1. Check that the degrees match between the two CRS elements.
        if self.new_elements.degree != parent.degree {
            return false;
        }
        // 2. Check that the linking proofs verify
        let ctxs: [&'static [u8]; 3] = [
            b"phase1 alpha proof",
            b"phase1 beta proof",
            b"phase1 x proof",
        ];
        let statements = [
            dlog::Statement {
                result: self.new_elements.raw.alpha_1,
                base: parent.raw.alpha_1,
            },
            dlog::Statement {
                result: self.new_elements.raw.beta_1,
                base: parent.raw.beta_1,
            },
            dlog::Statement {
                result: self.new_elements.raw.x_1[1],
                base: parent.raw.x_1[1],
            },
        ];
        let proofs = [
            self.linking_proof.alpha_proof,
            self.linking_proof.beta_proof,
            self.linking_proof.x_proof,
        ];
        if !ctxs
            .iter()
            .zip(statements.iter())
            .zip(proofs.iter())
            .all(|((c, &s), p)| dlog::verify(c, s, p))
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use rand_core::OsRng;

    use crate::group::F;

    /// The degree we use for tests.
    ///
    /// Keeping this small makes tests go faster.
    const D: usize = 2;

    fn make_crs(alpha: F, beta: F, x: F) -> RawCRSElements {
        RawCRSElements {
            alpha_1: G1::generator() * alpha,
            beta_1: G1::generator() * beta,
            beta_2: G2::generator() * beta,
            x_1: vec![
                G1::generator(),
                G1::generator() * x,
                G1::generator() * (x * x),
            ],
            x_2: vec![G2::generator(), G2::generator() * x],
            alpha_x_1: vec![G1::generator() * alpha, G1::generator() * (alpha * x)],
            beta_x_1: vec![G1::generator() * beta, G1::generator() * (beta * x)],
        }
    }

    fn non_trivial_crs() -> RawCRSElements {
        let alpha = F::rand(&mut OsRng);
        let beta = F::rand(&mut OsRng);
        let x = F::rand(&mut OsRng);

        make_crs(alpha, beta, x)
    }

    #[test]
    fn test_root_crs_is_valid() {
        let root = CRSElements::root(D);
        assert!(root.raw.validate().is_some());
    }

    #[test]
    fn test_nontrivial_crs_is_valid() {
        let crs = non_trivial_crs();
        assert!(crs.validate().is_some());
    }

    #[test]
    fn test_changing_alpha_makes_crs_invalid() {
        let mut crs = non_trivial_crs();
        crs.alpha_1 = G1::generator();
        assert!(crs.validate().is_none());
    }

    #[test]
    fn test_changing_beta_makes_crs_invalid() {
        let mut crs = non_trivial_crs();
        crs.beta_1 = G1::generator();
        assert!(crs.validate().is_none());
    }

    #[test]
    fn test_setting_zero_elements_makes_crs_invalid() {
        let alpha = F::rand(&mut OsRng);
        let beta = F::rand(&mut OsRng);
        let x = F::rand(&mut OsRng);

        let crs0 = make_crs(F::zero(), beta, x);
        assert!(crs0.validate().is_none());
        let crs1 = make_crs(alpha, F::zero(), x);
        assert!(crs1.validate().is_none());
        let crs2 = make_crs(alpha, beta, F::zero());
        assert!(crs2.validate().is_none());
    }

    #[test]
    fn test_bad_powers_of_x_makes_crs_invalid() {
        let alpha = F::rand(&mut OsRng);
        let beta = F::rand(&mut OsRng);
        let x = F::rand(&mut OsRng);
        let crs = RawCRSElements {
            alpha_1: G1::generator() * alpha,
            beta_1: G1::generator() * beta,
            beta_2: G2::generator() * beta,
            x_1: vec![
                G1::generator(),
                G1::generator() * x,
                G1::generator() * (x * x),
                // The important part
                G1::generator() * (x * x),
            ],
            x_2: vec![G2::generator(), G2::generator() * x],
            alpha_x_1: vec![G1::generator() * alpha, G1::generator() * (alpha * x)],
            beta_x_1: vec![G1::generator() * beta, G1::generator() * (beta * x)],
        };
        assert!(crs.validate().is_none());
    }

    #[test]
    fn test_contribution_produces_valid_crs() {
        let root = CRSElements::root(D);
        let contribution = Contribution::make(
            &mut OsRng,
            ContributionHash([0u8; CONTRIBUTION_HASH_SIZE]),
            &root,
        );
        assert!(contribution.new_elements.raw.validate().is_some());
    }

    #[test]
    fn test_contribution_is_linked_to_parent() {
        let root = CRSElements::root(D);
        let contribution = Contribution::make(
            &mut OsRng,
            ContributionHash([0u8; CONTRIBUTION_HASH_SIZE]),
            &root,
        );
        assert!(contribution.is_linked_to(&root));
    }

    #[test]
    fn test_can_calculate_contribution_hash() {
        let root = CRSElements::root(D);
        let contribution = Contribution::make(
            &mut OsRng,
            ContributionHash([0u8; CONTRIBUTION_HASH_SIZE]),
            &root,
        );
        assert_ne!(contribution.hash(), contribution.parent)
    }

    #[test]
    fn test_contribution_is_not_linked_to_itself() {
        let root = CRSElements::root(D);
        let contribution = Contribution::make(
            &mut OsRng,
            ContributionHash([0u8; CONTRIBUTION_HASH_SIZE]),
            &root,
        );
        assert!(!contribution.is_linked_to(&contribution.new_elements));
    }

    #[test]
    fn test_contribution_is_not_linked_if_degree_changes() {
        // Same elements, the latter just has more
        let root0 = CRSElements::root(D);
        let root1 = CRSElements::root(D + 1);
        let contribution = Contribution::make(
            &mut OsRng,
            ContributionHash([0u8; CONTRIBUTION_HASH_SIZE]),
            &root0,
        );
        assert!(!contribution.is_linked_to(&root1));
    }
}
