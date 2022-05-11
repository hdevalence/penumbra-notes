use anyhow::{anyhow, Result};
use penumbra_proto::{chain as pb, Protobuf};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "pb::NoteSource", into = "pb::NoteSource")]
pub enum NoteSource {
    Transaction { id: [u8; 32] },
    Genesis,
    FundingStreamReward { epoch_index: u64 },
}

const CODE_INDEX: usize = 23;

impl NoteSource {
    pub fn to_bytes(&self) -> [u8; 32] {
        match self {
            Self::Transaction { id } => *id,
            Self::Genesis => {
                let mut bytes = [0u8; 32];
                bytes[CODE_INDEX] = 1;
                bytes
            }
            Self::FundingStreamReward { epoch_index } => {
                let mut bytes = [0u8; 32];
                bytes[CODE_INDEX] = 2;
                bytes[24..].copy_from_slice(&epoch_index.to_le_bytes());
                bytes
            }
        }
    }
}

impl TryFrom<[u8; 32]> for NoteSource {
    type Error = anyhow::Error;
    fn try_from(bytes: [u8; 32]) -> Result<Self> {
        if bytes[..CODE_INDEX] != [0u8; CODE_INDEX][..] {
            Ok(Self::Transaction { id: bytes })
        } else {
            match (bytes[CODE_INDEX], &bytes[CODE_INDEX + 1..]) {
                (1, &[0, 0, 0, 0, 0, 0, 0, 0]) => Ok(Self::Genesis),
                (2, epoch_bytes) => {
                    let epoch_index =
                        u64::from_le_bytes(epoch_bytes.try_into().expect("slice is of length 8"));
                    Ok(Self::FundingStreamReward { epoch_index })
                }
                (code, data) => Err(anyhow!(
                    "unknown note source with code {} and data {:?}",
                    code,
                    data
                )),
            }
        }
    }
}

impl Protobuf<pb::NoteSource> for NoteSource {}

impl TryFrom<pb::NoteSource> for NoteSource {
    type Error = anyhow::Error;
    fn try_from(note_source: pb::NoteSource) -> Result<Self> {
        <[u8; 32]>::try_from(note_source.inner)
            .map_err(|_| anyhow!("expected 32 bytes"))?
            .try_into()
    }
}

impl From<NoteSource> for pb::NoteSource {
    fn from(note_source: NoteSource) -> Self {
        pb::NoteSource {
            inner: note_source.to_bytes().to_vec(),
        }
    }
}

impl std::fmt::Debug for NoteSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoteSource::Transaction { id } => {
                f.write_fmt(format_args!("NoteSource::Transaction({})", hex::encode(id)))
            }
            NoteSource::Genesis => f.write_fmt(format_args!("NoteSource::Genesis")),
            NoteSource::FundingStreamReward { epoch_index } => f.write_fmt(format_args!(
                "NoteSource::FundingStreamReward({})",
                epoch_index
            )),
        }
    }
}
