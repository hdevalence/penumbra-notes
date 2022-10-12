/// Snapshots maintain a point-in-time view of the underlying storage, suitable
/// for read-only access by multiple threads, i.e. RPC calls.
///
/// This is implemented as a wrapper around a [RocksDB snapshot](https://github.com/facebook/rocksdb/wiki/Snapshot)
/// with an associated JMT version number for the snapshot.
pub(crate) struct Snapshot {
    // TODO: the `'static` lifetime is a temporary hack and we'll need to find a workaround separately (tracked in #1512)
    rocksdb_snapshot: rocksdb::Snapshot<'static>,
    jmt_version: jmt::Version,
}

impl Snapshot {
    pub(crate) fn new(
        rocksdb_snapshot: rocksdb::Snapshot<'static>,
        jmt_version: jmt::Version,
    ) -> Self {
        Self {
            rocksdb_snapshot,
            jmt_version,
        }
    }

    pub fn get_raw(&self, key: String) -> Option<Vec<u8>> {
        self.rocksdb_snapshot.get(key).ok().flatten()
    }

    pub fn jmt_version(&self) -> jmt::Version {
        self.jmt_version
    }
}
