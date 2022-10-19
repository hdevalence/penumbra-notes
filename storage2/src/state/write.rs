use std::fmt::Debug;

use penumbra_proto::{Message, Protobuf};

pub trait StateWrite {
    /// Copy-on-write put
    fn put_raw(&mut self, key: String, value: Vec<u8>);

    /// Sets a domain type on the State.
    fn put<D, P>(&mut self, key: String, value: D)
    where
        D: Protobuf<P>,
        // TODO: does this get less awful if P is an associated type of D?
        P: Message + Default,
        P: From<D>,
        D: TryFrom<P> + Clone + Debug,
        <D as TryFrom<P>>::Error: Into<anyhow::Error>,
    {
        self.put_proto(key, P::from(value));
    }

    /// Puts a proto type on the State.
    fn put_proto<D, P>(&mut self, key: String, value: P)
    where
        D: Protobuf<P>,
        // TODO: does this get less awful if P is an associated type of D?
        P: Message + Default,
        P: From<D>,
        D: TryFrom<P> + Clone + Debug,
        <D as TryFrom<P>>::Error: Into<anyhow::Error>,
    {
        self.put_raw(key, value.encode_to_vec());
    }

    /// Delete a key from state.
    fn delete(&mut self, key: String);

    /// Delete a key from sidecar storage.
    fn delete_sidecar(&mut self, key: Vec<u8>);

    /// Put a key/value raw pair into non-consensus-critical ("sidecar") state.
    fn put_sidecar(&mut self, key: Vec<u8>, value: Vec<u8>);
}
