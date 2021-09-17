# Addresses and Keys

The key hierarchy is based on a modified [Zcash Sapling](https://zips.z.cash/protocol/protocol.pdf) design,
which we summarize here.  In contrast to Sapling, Penumbra's transaction system
includes support for [fuzzy message detection](./primitives/fmd.md), uses
Poseidon for hashing, and uses `decaf377` instead of Jubjub, so that it can be
used with the BLS12-377 curve instead of the BLS12-381 curve.

```mermaid
flowchart BT
    subgraph Address
        direction TB;
        d2[d];
        pk_d;
        cfk_d;
    end;
    subgraph D[Diversifier]
        d1[d];
    end;
    subgraph DTK[Detection Key]
        cdtk_d;
    end;
    subgraph IVK[Incoming Viewing Key]
        ivk;
    end;
    subgraph FVK[Full Viewing Key]
        direction TB;
        ak2[ak];
        nk;
        ovk2[ovk];
    end;
    subgraph PAK[Proof Authorizing Key]
        direction TB;
        ak1[ak];
        nsk2[nsk];
    end;
    subgraph ESK[Expanded Spending Key]
        direction TB;
        ask;
        nsk1[nsk];
        ovk1[ovk];
    end;
    subgraph SK[Spending Key]
        sk;
    end;

    sk --> ask;
    sk --> nsk1;
    sk --> ovk1;

    ask --> ak1;
    nsk1 --- nsk2;
    ovk1 --- ovk2;

    ak1 --- ak2;
    nsk2 --> nk;

    ak2 --> ivk;
    nk --> ivk;

    ivk --> pk_d;

    d1 --- d2;
    d1 --> pk_d;

    ivk --> cdtk_d;
    d1 --> cdtk_d;
    cdtk_d --> cfk_d;
```

All addresses and keys are ultimately derived from a secret *spending key* $sk$, which is a random 32-byte string. From this *spending key* $sk$, we derive several other keys, each described in more detail in its own section:

* an expanded form of the spending key called the [*expanded spending key*](./addresses_keys/expanded_spending_keys.md) which has components used to derive *viewing keys* and the *proof authorization key* as described below,
* a [*proof authorization key*](./addresses_keys/proof_authorization_keys.md), which lets the holder spend notes associated with the *spending key*,
* [*viewing keys*](./addresses_keys/viewing_keys.md) which allow the holder to identify but not spend notes associated with the *spending key*,
* [*addresses*](./addresses_keys/addresses.md), which can be shared in order to receive payments.
