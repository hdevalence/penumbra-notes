use std::error::Error;

use penumbra_asset::asset::Id as AssetId;
use sqlx::{Decode, Encode, Postgres, Type};

/// An extension trait to make it easier to implement serialization for existing Penumbra types.
///
/// Types that implement this trait can then be shoved into [Sql] and passed along
/// to the various sqlx functions.
pub trait SqlExt: Clone + Sized {
    type SqlT;

    fn to_sql_type(&self) -> Self::SqlT;
    fn from_sql_type(value: Self::SqlT) -> anyhow::Result<Self>;
}

/// A wrapper over `T` allowing for SQL serialization and deserialization.
///
/// When `T` implements [SqlExt] then this type will be encodeable and decodeable
/// from a Postgres database.
pub struct Sql<T>(T);

impl<T> Sql<T> {
    #[allow(dead_code)]
    pub fn into(self) -> T {
        self.0
    }
}

impl<T> From<T> for Sql<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<'q, T> Encode<'q, Postgres> for Sql<T>
where
    T: SqlExt,
    T::SqlT: Encode<'q, Postgres>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <Postgres as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        <T as SqlExt>::to_sql_type(&self.0).encode_by_ref(buf)
    }
}

impl<'q, T> Decode<'q, Postgres> for Sql<T>
where
    T: SqlExt,
    T::SqlT: Decode<'q, Postgres>,
{
    fn decode(
        value: <Postgres as sqlx::database::HasValueRef<'q>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let sql_t = <T as SqlExt>::SqlT::decode(value)?;
        let t = T::from_sql_type(sql_t)
            .map_err(|e| Box::<dyn Error + Send + Sync + 'static>::from(e))?;
        Ok(Sql(t))
    }
}

impl<T> Type<Postgres> for Sql<T>
where
    T: SqlExt,
    T::SqlT: Type<Postgres>,
{
    fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
        <[u8; 32]>::type_info()
    }
}

impl SqlExt for AssetId {
    type SqlT = [u8; 32];

    fn to_sql_type(&self) -> Self::SqlT {
        self.to_bytes()
    }

    fn from_sql_type(value: Self::SqlT) -> anyhow::Result<Self> {
        Ok(AssetId::try_from(value.as_slice())?)
    }
}
