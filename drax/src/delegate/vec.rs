use crate::delegate::primitive::size_var_int;
use crate::prelude::{
    DraxReadExt, DraxResult, DraxWriteExt, PacketComponent, Size, TransportError,
};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A delegate struct which encodes and decodes a `Vec<u8>` type.
///
/// This delegate instructs the reader to read the entirety of the remaining bytes
/// into the `Vec<u8>` type.
///
/// ```rust
/// # use drax::prelude::*;
/// # use std::io::Cursor;
/// # async fn test() -> DraxResult<()> {
/// let mut cursor = Cursor::new(vec![]);
/// cursor.encode_component::<ByteDrain>(&vec![10, 20, 30]).await?;
/// cursor.set_position(0);
/// let back = cursor.decode_component::<ByteDrain>().await?;
/// assert_eq!(back, vec![10, 20, 30]);
/// # Ok(())
/// # }
/// ```
pub struct ByteDrain;

impl<C: Send + Sync> PacketComponent<C> for ByteDrain {
    type ComponentType = Vec<u8>;

    decode!(read {
        let mut bytes = vec![];
        read.read_to_end(&mut bytes).await?;
        Ok(bytes)
    });

    encode!(component_ref, write {
        write.write_all(component_ref).await?;
    });

    fn size(component_ref: &Self::ComponentType, _: &mut C) -> DraxResult<Size> {
        Ok(Size::Dynamic(component_ref.len()))
    }
}

/// A delegate struct which encodes and decodes a `[N; u8]` type.
///
/// This differs from the `[T; N]` implementation in that it optimizes the
/// read and write operations since the length is also the remaining bytes
/// to be read.
#[cfg(feature = "slices")]
pub struct SliceU8<const N: usize>;

#[cfg(feature = "slices")]
impl<C: Send + Sync, const N: usize> PacketComponent<C> for SliceU8<N> {
    type ComponentType = [u8; N];

    decode!(read {
        let mut buf = [0; N];
        read.read_exact(&mut buf).await?;
        Ok(buf)
    });

    encode!(component_ref, write {
        write.write_all(component_ref).await?;
    });

    fn size(_: &Self::ComponentType, _: &mut C) -> DraxResult<Size> {
        Ok(Size::Constant(N))
    }
}

#[cfg(feature = "slices")]
impl<C: Send + Sync, T, const N: usize> PacketComponent<C> for [T; N]
where
    T: PacketComponent<C>,
{
    type ComponentType = [T::ComponentType; N];

    decode!(read, context {
        let mut arr: [MaybeUninit<T::ComponentType>; N] = MaybeUninit::uninit_array();
        for i in &mut arr {
            *i = MaybeUninit::new(T::decode(context, read).await?);
        }
        Ok(arr.map(|x| unsafe { x.assume_init() }))
    });

    encode!(component_ref, write, context {
        for x in component_ref {
            T::encode(x, context, write).await?;
        }
    });

    fn size(component_ref: &Self::ComponentType, context: &mut C) -> DraxResult<Size> {
        let mut dynamic_counter = 0;
        for item in component_ref {
            match T::size(item, context)? {
                Size::Constant(x) => return Ok(Size::Constant(x * N)),
                Size::Dynamic(x) => dynamic_counter += x,
            }
        }
        Ok(Size::Dynamic(dynamic_counter))
    }
}

/// A delegate struct which encodes and decodes a `Vec<u8>` type.
///
/// Similar to the `SliceU8` delegate, this optimizes the read and write operations
/// since the length is also the remaining bytes to be read.
pub struct VecU8;

impl<C: Send + Sync> PacketComponent<C> for VecU8 {
    type ComponentType = Vec<u8>;

    decode!(read {
        let len = read.read_var_int().await?;
        let mut buf = vec![0u8; len as usize];
        read.read_exact(&mut buf).await?;
        Ok(buf)
    });

    encode!(component_ref, write {
        write.write_var_int(component_ref.len() as i32).await?;
        write.write_all(component_ref).await?;
    });

    fn size(component_ref: &Self::ComponentType, _: &mut C) -> DraxResult<Size> {
        Ok(Size::Dynamic(
            component_ref.len() + size_var_int(component_ref.len() as i32),
        ))
    }
}

impl<C: Send + Sync, T> PacketComponent<C> for Vec<T>
where
    T: PacketComponent<C>,
{
    type ComponentType = Vec<T::ComponentType>;

    decode!(read, context {
        let len = read.read_var_int().await?;
        let mut vec = Vec::with_capacity(len as usize);
        for _ in 0..len {
            vec.push(T::decode(context, read).await?);
        }
        Ok(vec)
    });

    encode!(component_ref, write, context {
        write.write_var_int(component_ref.len() as i32).await?;
        for item in component_ref {
            T::encode(item, context, write).await?;
        }
    });

    fn size(component_ref: &Self::ComponentType, context: &mut C) -> DraxResult<Size> {
        let var_int_size = size_var_int(component_ref.len() as i32);
        let mut dynamic_counter = var_int_size;
        for item in component_ref {
            match T::size(item, context)? {
                Size::Constant(x) => {
                    return Ok(Size::Dynamic((x * component_ref.len()) + var_int_size));
                }
                Size::Dynamic(x) => dynamic_counter += x,
            }
        }
        Ok(Size::Dynamic(dynamic_counter))
    }
}

/// A delegate struct which limits the size of a `Vec<T>` when encoding/decoding to the
/// given constant limit.
pub struct LimitedVec<T, const N: usize>(PhantomData<T>);

impl<T, C: Send + Sync, const N: usize> PacketComponent<C> for LimitedVec<T, N>
where
    T: PacketComponent<C>,
{
    type ComponentType = Vec<T::ComponentType>;

    decode!(read, context {
        let vec_size = read.read_var_int().await?;
        let lim = N as i32;
        println!("lim {}, vec size {}", lim, vec_size);
        if vec_size > lim {
            return TransportError::limit_exceeded(lim, vec_size, "decoding vec");
        }

        let mut vec = Vec::with_capacity(vec_size as usize);
        for _ in 0..vec_size {
            vec.push(T::decode(context, read).await?);
        }
        Ok(vec)
    });

    encode!(component_ref, write, context {
        let len = component_ref.len() as i32;
        let lim = N as i32;

        if len > lim {
            return TransportError::limit_exceeded(lim, len, "encoding vec");
        }

        Vec::<T>::encode(component_ref, context, write).await?;
    });

    fn size(component_ref: &Self::ComponentType, context: &mut C) -> DraxResult<Size> {
        Vec::<T>::size(component_ref, context)
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::{
        ByteDrain, DraxReadExt, DraxWriteExt, LimitedVec, SliceU8, VarInt, VecU8,
    };
    use std::io::Cursor;
    use tokio_test::assert_err;

    #[tokio::test]
    pub async fn byte_drain_sanity() -> crate::prelude::DraxResult<()> {
        let bytes = vec![10, 20, 30];

        let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![]);

        cursor.encode_component::<ByteDrain>(&bytes).await?;

        assert_eq!(cursor.into_inner(), vec![10, 20, 30]);
        Ok(())
    }

    #[tokio::test]
    pub async fn slice_u8_sanity() -> crate::prelude::DraxResult<()> {
        type UsedSliceType = SliceU8<3>;

        let bytes = [10, 20, 30];

        let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![]);

        cursor.encode_component::<UsedSliceType>(&bytes).await?;
        cursor.set_position(0);

        assert_eq!(cursor.decode_component::<UsedSliceType>().await?, bytes);
        Ok(())
    }

    #[tokio::test]
    pub async fn slice_sanity() -> crate::prelude::DraxResult<()> {
        type UsedSliceType = [VarInt; 3];

        let slice = [10, 20, 30];

        let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![]);

        cursor.encode_component::<UsedSliceType>(&slice).await?;
        cursor.set_position(0);

        assert_eq!(cursor.decode_component::<UsedSliceType>().await?, slice);
        Ok(())
    }

    #[tokio::test]
    pub async fn vec_u8_sanity() -> crate::prelude::DraxResult<()> {
        let bytes = vec![10, 20, 30];

        let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![]);

        cursor.encode_component::<VecU8>(&bytes).await?;
        cursor.set_position(0);

        assert_eq!(cursor.decode_component::<VecU8>().await?, bytes);
        Ok(())
    }

    #[tokio::test]
    pub async fn vec_sanity() -> crate::prelude::DraxResult<()> {
        let bytes = vec![10, 20, 30];

        let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![]);

        cursor.encode_component::<Vec<VarInt>>(&bytes).await?;
        cursor.set_position(0);

        assert_eq!(cursor.decode_component::<Vec<VarInt>>().await?, bytes);
        Ok(())
    }

    #[tokio::test]
    pub async fn test_limited_vec_failure() -> crate::prelude::DraxResult<()> {
        let bytes = vec![10, 20, 30];

        let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![]);

        cursor
            .encode_component::<LimitedVec<VarInt, 3>>(&bytes)
            .await?;
        cursor.set_position(0);

        assert_err!(cursor.decode_component::<LimitedVec<VarInt, 2>>().await);
        Ok(())
    }
}
