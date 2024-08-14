use std::mem::size_of;

use crate::prelude::{DraxReadExt, DraxResult, DraxWriteExt, PacketComponent, Size};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

mod var_num {
    use std::future::Future;
    use std::marker::PhantomPinned;
    use std::pin::Pin;
    use std::task::{ready, Context, Poll};

    use crate::prelude::{AsyncRead, AsyncWrite, DraxResult, TransportError};
    use pin_project_lite::pin_project;
    use tokio::io::ReadBuf;

    macro_rules! declare_var_num_ext {
        (
            $typing:ty,
            $sub_typing:ty,
            $size_fn:ident,
            $read_fn:ident,
            $read_struct:ident,
            $write_fn:ident,
            $write_struct:ident,
            $bit_limit:literal,
            $and_check:literal
        ) => {
            pub fn $size_fn(var_num: $typing) -> usize {
                let mut temp: $sub_typing = var_num as $sub_typing;
                let mut size = 0;
                loop {
                    if (temp & $and_check) == 0 {
                        return size + 1;
                    }
                    size += 1;
                    temp = temp.overflowing_shr(7).0;
                }
            }

            pub(crate) fn $read_fn<A>(reader: &mut A) -> $read_struct<A>
            where
                A: AsyncRead + Unpin + ?Sized,
            {
                $read_struct {
                    reader,
                    value: 0,
                    bit_offset: 0,
                    _pin: PhantomPinned,
                }
            }

            pin_project! {
                #[derive(Debug)]
                #[must_use = "futures do nothing unless you `.await` or poll them"]
                pub struct $read_struct<'a, A: ?Sized> {
                    reader: &'a mut A,
                    value: $typing,
                    bit_offset: u32,
                    // Make this future `!Unpin` for compatibility with async trait methods.
                    #[pin]
                    _pin: PhantomPinned,
                }
            }

            impl<A> Future for $read_struct<'_, A>
            where
                A: AsyncRead + Unpin + ?Sized,
            {
                type Output = DraxResult<$typing>;

                fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<DraxResult<$typing>> {
                    let me = self.project();

                    loop {
                        if *me.bit_offset >= $bit_limit {
                            return Poll::Ready(Err(TransportError::VarNumTooLarge));
                        };

                        let mut inner = [0u8; 1];
                        let mut buf = ReadBuf::new(inner.as_mut());
                        ready!(Pin::new(&mut *me.reader).poll_read(cx, &mut buf))?;
                        if buf.filled().len() == 0 {
                            return Poll::Ready(Err(TransportError::EOF));
                        }
                        let byte = buf.filled()[0];
                        *me.value |= <$typing>::from(byte & 0b0111_1111)
                            .overflowing_shl(*me.bit_offset)
                            .0;
                        *me.bit_offset += 7;
                        if byte & 0b1000_0000 == 0 {
                            return Poll::Ready(Ok(*me.value));
                        }
                    }
                }
            }

            pub(crate) fn $write_fn<A>(writer: &mut A, value: $typing) -> $write_struct<A>
            where
                A: AsyncWrite + Unpin + ?Sized,
            {
                $write_struct {
                    writer,
                    value,
                    _pin: PhantomPinned,
                }
            }

            pin_project! {
                #[derive(Debug)]
                #[must_use = "futures do nothing unless you `.await` or poll them"]
                pub struct $write_struct<'a, A: ?Sized> {
                    writer: &'a mut A,
                    value: $typing,
                    // Make this future `!Unpin` for compatibility with async trait methods.
                    #[pin]
                    _pin: PhantomPinned,
                }
            }

            impl<A> Future for $write_struct<'_, A>
            where
                A: AsyncWrite + Unpin + ?Sized,
            {
                type Output = DraxResult<()>;

                fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<DraxResult<()>> {
                    let me = self.project();

                    let mut value: $sub_typing = *me.value as $sub_typing;
                    loop {
                        if (value & $and_check) == 0 {
                            ready!(Pin::new(&mut *me.writer).poll_write(cx, &[value as u8]))?;
                            return Poll::Ready(Ok(()));
                        }
                        ready!(Pin::new(&mut *me.writer)
                            .poll_write(cx, &[(value & 0x7F | 0x80) as u8]))?;
                        value = value.overflowing_shr(7).0;
                        *me.value = value.try_into().unwrap();
                    }
                }
            }
        };
    }

    declare_var_num_ext!(
        i32,
        u32,
        size_var_int,
        read_var_int,
        ReadVarInt,
        write_var_int,
        WriteVarInt,
        35,
        0xFFFFFF80u32
    );

    declare_var_num_ext!(
        i64,
        u64,
        size_var_long,
        read_var_long,
        ReadVarLong,
        write_var_long,
        WriteVarLong,
        70,
        0xFFFFFFFFFFFFFF80u64
    );
}
pub(crate) use var_num::{read_var_int, read_var_long, write_var_int, write_var_long};
pub use var_num::{
    size_var_int, size_var_long, ReadVarInt, ReadVarLong, WriteVarInt, WriteVarLong,
};

macro_rules! define_primitive_bind {
    ($($prim:ty),*) => {
        $(
            impl<C: Send + Sync> PacketComponent<C> for $prim {
                type ComponentType = $prim;

                decode!(read {
                    let mut buf = [0; size_of::<Self>()];
                    read.read_exact(&mut buf).await?;
                    Ok(Self::from_be_bytes(buf))
                });

                encode!(component_ref, write {
                    write.write_all(component_ref.to_be_bytes().as_ref()).await?;
                });

                fn size(_: &Self, __: &mut C) -> DraxResult<Size> {
                    Ok(Size::Constant(size_of::<Self>()))
                }
            }
        )*
    }
}

define_primitive_bind!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

impl<C: Send + Sync> PacketComponent<C> for () {
    type ComponentType = ();

    decode!(_read Ok(()));

    encode!(_component_ref, _write);

    fn size(_: &Self::ComponentType, _: &mut C) -> DraxResult<Size> {
        Ok(Size::Constant(0))
    }
}

impl<C: Send + Sync> PacketComponent<C> for bool {
    type ComponentType = bool;

    decode!(read Ok(read.read_u8().await? != 0x0));

    encode!(component_ref, write write.write_u8(if *component_ref { 0x1 } else { 0x0 }).await?);

    fn size(_: &Self::ComponentType, _: &mut C) -> DraxResult<Size> {
        Ok(Size::Constant(1))
    }
}

/// A delegate struct which encodes and decodes an `i32` type.
///
/// This delegate will attempt to encode the integer using the smallest possible
/// number of bytes. The VarInt uses a bit mask to describe when the bytes are
/// fully read.
pub struct VarInt;

impl<C: Send + Sync> PacketComponent<C> for VarInt {
    type ComponentType = i32;

    decode!(read read.read_var_int().await);

    encode!(component_ref, write write.write_var_int(*component_ref).await?);

    fn size(input: &Self::ComponentType, _: &mut C) -> DraxResult<Size> {
        Ok(Size::Dynamic(size_var_int(*input)))
    }
}

/// A delegate struct which encodes and decodes a `i64` type.
///
/// This delegate will attempt to encode the long using the smallest possible
/// number of bytes. The VarLong uses a bit mask to describe when the bytes are
/// fully read.
pub struct VarLong;

impl<C: Send + Sync> PacketComponent<C> for VarLong {
    type ComponentType = i64;

    decode!(read read.read_var_long().await);

    encode!(component_ref, write write.write_var_long(*component_ref).await?);

    fn size(input: &Self::ComponentType, _: &mut C) -> DraxResult<Size> {
        Ok(Size::Dynamic(size_var_long(*input)))
    }
}

#[cfg(feature = "uuid")]
impl<C: Send + Sync> PacketComponent<C> for Uuid {
    type ComponentType = Uuid;

    decode!(read {
        let mut buf = [0; 16];
        read.read_exact(&mut buf).await?;
        let uuid = Uuid::from_slice(&buf)?;
        Ok(uuid)
    });

    encode!(component_ref, write {
        write.write_all(component_ref.as_bytes()).await?;
    });

    fn size(_: &Self::ComponentType, _: &mut C) -> DraxResult<Size> {
        Ok(Size::Constant(size_of::<u64>() * 2))
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::{DraxReadExt, DraxResult, DraxWriteExt};
    use std::io::Cursor;
    use uuid::Uuid;

    macro_rules! primitive_tests {
        ($testable_ty:ty; $testable_value:expr; $test_ident:ident) => {
            #[tokio::test]
            async fn $test_ident() -> DraxResult<()> {
                let expected = $testable_value;
                let mut cursor = Cursor::new(vec![]);
                cursor.encode_own_component(&expected).await?;
                cursor.set_position(0);
                let back = cursor.decode_own_component::<$testable_ty>().await?;
                assert_eq!(back, expected);
                Ok(())
            }
        };
        ($op:tt .. $($testable_pre:ty, $testable_post:ty, $test_ident:ident);*) => {
            $(primitive_tests!($testable_post; { <$testable_pre>::MAX as $testable_post $op 10 }; $test_ident);)*
        };
    }

    macro_rules! var_int_tests {
        () => {
            vec![
                (25, vec![25]),
                (55324, vec![156, 176, 3]),
                (-8877777, vec![175, 146, 226, 251, 15]),
                (2147483647, vec![255, 255, 255, 255, 7]),
                (-2147483648, vec![128, 128, 128, 128, 8]),
            ]
        };
    }

    #[tokio::test]
    async fn test_read_var_int() -> DraxResult<()> {
        for attempt in var_int_tests!() {
            let mut cursor = Cursor::new(attempt.1);
            let result = cursor.read_var_int().await?;
            assert_eq!(result, attempt.0);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_write_var_int() -> DraxResult<()> {
        for attempt in var_int_tests!() {
            let mut cursor = Cursor::new(vec![]);
            cursor.write_var_int(attempt.0).await?;
            assert_eq!(cursor.into_inner(), attempt.1);
        }
        Ok(())
    }

    primitive_tests!(u8; 10; test_u8);
    primitive_tests!(+ ..
        u8, u16, test_u16;
        u16, u32, test_u32;
        u32, u64, test_u64
    );
    primitive_tests!(i8; 10; test_i8);
    primitive_tests!(- ..
        i8, i16, test_i16;
        i16, i32, test_i32;
        i32, i64, test_i64
    );
    primitive_tests!(f32; 30.40; test_f32);
    primitive_tests!(f64; { f32::MAX as f64 + 30.40 }; test_f64);

    #[cfg(feature = "uuid")]
    #[tokio::test]
    async fn test_uuid() -> DraxResult<()> {
        let expected = Uuid::new_v4();
        let mut cursor = Cursor::new(vec![]);
        cursor.encode_component::<Uuid>(&expected).await?;
        cursor.set_position(0);
        let back = cursor.decode_component::<Uuid>().await?;
        assert_eq!(back, expected);
        Ok(())
    }
}
