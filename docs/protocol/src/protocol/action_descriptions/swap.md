# Swap Descriptions

Each swap contains a SwapBody and a zk-SNARK swap proof.

## Swap zk-SNARK Statements

The swap proof demonstrates the properties enumerated below for the private witnesses known by the prover:

* Swap plaintext which consists of:
  * Trading pair, which consists of two asset IDs  $ID_1, ID_2 \isin \mathbb F_q$
  * Fee value which consists of an amount $v_f$ interpreted as an $\mathbb F_q$ constrained to fit in 128 bits, and an asset ID $ID_{v_f} \isin \mathbb F_q$
  * Input amount of the first asset $v_1$ interpreted as an $\mathbb F_q$ constrained to fit in 128 bits
  * Input amount of the second asset $v_2$ interpreted as an $\mathbb F_q$ constrained to fit in 128 bits
  * `Rseed`, interpreted as an $\mathbb F_q$
  * Diversified basepoint $B_d \isin \mathbb G$ corresponding to the claim address
  * Transmission key $pk_d \isin \mathbb G$ corresponding to the claim address
  * Clue key $\mathsf{ck_d} \isin \mathbb F_q$ corresponding to the claim address
* Fee blinding factor $\widetilde{v_f} \isin \mathbb F_r$ used to blind the fee commitment

And the corresponding public inputs:

* Balance commitment $cv \isin G$ to the value balance
* Fee commitment $cv_f \isin G$ to the value of the fee
* Swap commitment $scm \isin \mathbb F_q$

### Swap Commitment Integrity

The zk-SNARK certifies that the public input swap commitment $scm$ was derived as:

$scm_{inner} = hash_4(ds, (ID_1, ID_2, v_1, v_2))$

$scm = hash_7(ds, (rseed, v_f, ID_{v_f}, B_d, pk_d, \mathsf{ck_d}, scm_{inner}))$.

using the above witnessed values and where `ds` is a constant domain separator:

`ds = from_le_bytes(BLAKE2b-512(b"penumbra.swap")) mod q`

### Fee Commitment Integrity

The zk-SNARK certifies that the public input fee commitment $cv_f$ was derived from the witnessed values as:

$cv_f = [-v_f] G_{v_f} + [\widetilde{v_f}] G_{\widetilde{v}}$

where $G_{\widetilde{v}}$ is a constant generator and $G_{v_f}$ is an asset-specific generator point derived in-circuit from the asset ID $ID_{v_f}$ as described in [Value Commitments](../../protocol/value_commitments.md).

### Balance Commitment Integrity

The zk-SNARK certifies that the total public input balance commitment $cv$ was derived from the witnessed values as:

$cv = [-v_1] G_1 + [-v_2] G_2 + cv_f$

where the first two terms are from the input amounts and assets, with the corresponding asset-specific generators $G_1, G_2$ derived in-circuit as described in [Value Commitments](../../protocol/value_commitments.md), and $cv_f$ is the fee commitment.
