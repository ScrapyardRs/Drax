use crate::error::DraxResult;
use crate::prelude::{PacketComponent, Size, VecU8};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// A delegate struct which encodes and decodes a `serde::Serialize` and `serde::Deserialize` value.
///
/// # Example
/// ```rust
/// # use std::collections::HashMap;
/// # use drax::prelude::*;
/// # use std::io::Cursor;
/// #[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Debug)]
/// struct ExampleStruct {
///     example: String,
///     number: i32,
///     map: HashMap<String, i32>,
/// }
///
/// # #[tokio::test]
/// # async fn test() -> DraxResult<()> {
/// let example = ExampleStruct {
///     example: "test string".to_string(),
///     number: 10,
///     map: HashMap::from([("example".to_string(), 10), ("example2".to_string(), 20)]),
/// };
///
/// let mut cursor = Cursor::new(vec![]);
/// cursor.encode_component::<JsonDelegate<_>>(&example).await?;
/// cursor.set_position(0);
/// let back = cursor.decode_component::<JsonDelegate<_>>().await?;
/// assert_eq!(example, back);
/// # Ok(())
/// # }
/// ```
pub struct JsonDelegate<T> {
    _phantom_t: PhantomData<T>,
}

impl<C: Send + Sync, T> PacketComponent<C> for JsonDelegate<T>
where
    T: for<'de> Deserialize<'de>,
    T: Serialize + Send + Sync,
{
    type ComponentType = T;

    decode!(read, context {
        let bytes = VecU8::decode(context, read).await?;
        let value: T = serde_json::from_slice(&bytes)?;
        Ok(value)
    });

    encode!(component_ref, write, context {
        let bytes = serde_json::to_vec(&component_ref)?;
        VecU8::encode(&bytes, context, write).await?;
    });

    fn size(input: &Self::ComponentType, context: &mut C) -> DraxResult<Size> {
        VecU8::size(&serde_json::to_vec(&input)?, context)
    }
}
