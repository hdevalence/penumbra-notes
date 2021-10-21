pub const OVK_LEN_BYTES: usize = 32;

/// Allows viewing outgoing notes, i.e., notes sent from the spending key this
/// key is derived from.
#[derive(Clone)]
pub struct OutgoingViewingKey(pub(super) [u8; OVK_LEN_BYTES]);
