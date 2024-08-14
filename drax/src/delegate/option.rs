use crate::prelude::{DraxResult, PacketComponent, Size};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A delegate struct which encodes and decodes an `Option<T>` type.
///
/// This delegate encodes the existence of an `Option<T>` as a single byte which is either
/// 0x0 or 0x1.
///
/// # Exists Example
/// ```rust
/// # use drax::prelude::*;
/// # use std::io::Cursor;
/// # async fn test() -> DraxResult<()> {
/// let mut cursor = Cursor::new(vec![]);
/// cursor.encode_component::<Maybe<i32>>(&Some(10)).await?;
/// assert_eq!(cursor.clone().into_inner(), vec![1, 0, 0, 0, 10]);
/// cursor.set_position(0);
/// let back = cursor.decode_component::<Maybe<i32>>().await?;
/// assert_eq!(back, Some(10));
/// # Ok(())
/// # }
/// ```
///
/// # Does Not Exist Example
/// ```rust
/// # use drax::prelude::*;
/// # use std::io::Cursor;
/// # async fn test() -> DraxResult<()> {
/// let mut cursor = Cursor::new(vec![0]);
/// let back = cursor.decode_component::<Maybe<i32>>().await?;
/// assert_eq!(back, None);
/// # Ok(())
/// # }
/// ```
pub struct Maybe<T> {
    _phantom_t: T,
}

impl<C: Send + Sync, T: PacketComponent<C>> PacketComponent<C> for Maybe<T> {
    type ComponentType = Option<T::ComponentType>;

    decode!(read, context {
        Ok(if read.read_u8().await? == 0x0 {
            None
        } else {
            Some(T::decode(context, read).await?)
        })
    });

    encode!(component_ref, write, context {
        write
            .write_u8(if component_ref.is_some() { 1 } else { 0 })
            .await?;
        if let Some(value) = component_ref {
            T::encode(value, context, write).await?;
        }
    });

    fn size(input: &Self::ComponentType, ctx: &mut C) -> DraxResult<Size> {
        Ok(if let Some(value) = input {
            Size::Constant(1) + T::size(value, ctx)?
        } else {
            Size::Constant(1)
        })
    }
}
