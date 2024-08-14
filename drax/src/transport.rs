use crate::delegate::primitive;
use crate::delegate::primitive::{ReadVarInt, ReadVarLong, WriteVarInt, WriteVarLong};
use crate::prelude::{AsyncRead, AsyncWrite, DraxResult};

/// Declares the size in bytes of a packet component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Size {
    /// The size of the packet is dynamic and cannot be optimized, the size must be calculated
    /// for each packet.
    Dynamic(usize),
    /// The size of the packet is constant and can be optimized by caching the size of the packet
    /// type.
    Constant(usize),
}

impl std::ops::Add for Size {
    type Output = Size;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Size::Dynamic(x), Size::Dynamic(y))
            | (Size::Dynamic(x), Size::Constant(y))
            | (Size::Constant(x), Size::Dynamic(y)) => Size::Dynamic(x + y),
            (Size::Constant(x), Size::Constant(y)) => Size::Constant(x + y),
        }
    }
}

impl std::ops::Add<usize> for Size {
    type Output = Size;

    fn add(self, rhs: usize) -> Self::Output {
        match self {
            Size::Dynamic(x) | Size::Constant(x) => Size::Dynamic(x + rhs),
        }
    }
}

/// Defines a structure that can be encoded and decoded.
pub trait PacketComponent<C: Send + Sync> {
    /// The type which the packet component is responsible for
    /// representing during reading and writing.
    type ComponentType: Sized + Send + Sync;

    /// Decodes the packet component from the given reader.
    async fn decode<A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &mut C,
        read: &mut A,
    ) -> DraxResult<Self::ComponentType>;

    /// Encodes the packet component to the given writer.
    async fn encode<A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &Self::ComponentType,
        context: &mut C,
        write: &mut A,
    ) -> DraxResult<()>;

    /// Calculates the size of the packet component.
    fn size(input: &Self::ComponentType, context: &mut C) -> DraxResult<Size>;
}

/// An extension trait which allows for quickly accessing component reading from
/// tokio AsyncRead types.
pub trait DraxReadExt {
    fn read_var_int(&mut self) -> ReadVarInt<'_, Self>;

    fn read_var_long(&mut self) -> ReadVarLong<'_, Self>;

    async fn decode_component<P: PacketComponent<()> + Sized>(
        &mut self,
    ) -> DraxResult<P::ComponentType>;

    async fn decode_own_component<P: PacketComponent<(), ComponentType = P> + Sized>(
        &mut self,
    ) -> DraxResult<P>;
}

impl<T> DraxReadExt for T
where
    T: AsyncRead + Unpin + Send + Sync + ?Sized,
{
    fn read_var_int(&mut self) -> ReadVarInt<'_, Self> {
        primitive::read_var_int(self)
    }

    fn read_var_long(&mut self) -> ReadVarLong<'_, Self> {
        primitive::read_var_long(self)
    }

    async fn decode_component<P: PacketComponent<()> + Sized>(
        &mut self,
    ) -> DraxResult<P::ComponentType> {
        P::decode(&mut (), self).await
    }

    async fn decode_own_component<P: PacketComponent<(), ComponentType = P> + Sized>(
        &mut self,
    ) -> DraxResult<P> {
        P::decode(&mut (), self).await
    }
}

/// An extension trait which allows for quickly accessing component writing to
/// tokio AsyncWrite types.
pub trait DraxWriteExt {
    fn write_var_int(&mut self, value: i32) -> WriteVarInt<'_, Self>;

    fn write_var_long(&mut self, value: i64) -> WriteVarLong<'_, Self>;

    async fn encode_component<P: PacketComponent<()>>(
        &mut self,
        component: &P::ComponentType,
    ) -> DraxResult<()>;

    async fn encode_own_component<P: PacketComponent<(), ComponentType = P>>(
        &mut self,
        component: &P,
    ) -> DraxResult<()>;
}

impl<T> DraxWriteExt for T
where
    T: AsyncWrite + Unpin + Send + Sync + ?Sized,
{
    fn write_var_int(&mut self, value: i32) -> WriteVarInt<'_, Self> {
        primitive::write_var_int(self, value)
    }

    fn write_var_long(&mut self, value: i64) -> WriteVarLong<'_, Self> {
        primitive::write_var_long(self, value)
    }

    async fn encode_component<P: PacketComponent<()>>(
        &mut self,
        component: &P::ComponentType,
    ) -> DraxResult<()> {
        P::encode(component, &mut (), self).await
    }

    async fn encode_own_component<P: PacketComponent<(), ComponentType = P>>(
        &mut self,
        component: &P,
    ) -> DraxResult<()> {
        P::encode(component, &mut (), self).await
    }
}

#[cfg(feature = "context")]
pub mod context {
    use crate::prelude::{AsyncRead, AsyncWrite, DraxResult, PacketComponent};

    /// A wrapper around a writer to streamline the process of encoding packet components
    /// using a specific context.
    ///
    /// ```rust
    /// # use drax::prelude::*;
    /// # use std::io::Cursor;
    /// # async fn test() -> DraxResult<()> {
    /// let mut cursor = Cursor::new(vec![]);
    /// cursor.writer_context(&mut ()).encode_own_component::<String>(&"test string".to_string()).await?;
    /// cursor.set_position(0);
    /// let back = cursor.decode_own_component::<String>().await?;
    /// assert_eq!(back, "test string");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// The provided example does not necessarily demonstrate the best use-case for this system
    /// but instead is meant to demonstrate how to emit context when encoding packets.
    pub struct ContextWrappedWriter<'a, W: AsyncWrite + Unpin + Send + Sync + ?Sized, C: Send + Sync>(
        &'a mut W,
        &'a mut C,
    );

    impl<W: AsyncWrite + Unpin + Send + Sync + ?Sized, C: Send + Sync> ContextWrappedWriter<'_, W, C> {
        pub async fn encode_component<P: PacketComponent<C>>(
            &mut self,
            component: &P::ComponentType,
        ) -> DraxResult<()> {
            P::encode(component, self.1, self.0).await
        }

        pub async fn encode_own_component<P: PacketComponent<C, ComponentType = P>>(
            &mut self,
            component: &P,
        ) -> DraxResult<()> {
            P::encode(component, self.1, self.0).await
        }
    }

    /// A wrapper around a reader to streamline the process of decoding packet components
    /// using a specific context.
    ///
    /// ```rust
    /// # use drax::prelude::*;
    /// # use std::io::Cursor;
    /// # async fn test() -> DraxResult<()> {
    /// let mut cursor = Cursor::new(vec![]);
    /// cursor.encode_component::<String>(&"test string".to_string()).await?;
    /// cursor.set_position(0);
    /// let back = cursor.reader_context(&mut ()).decode_component::<String>().await?;
    /// assert_eq!(back, "test string");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// The provided example does not necessarily demonstrate the best use-case for this system
    /// but instead is meant to demonstrate how to emit context when decoding packets.
    pub struct ContextWrappedReader<'a, R: AsyncRead + Unpin + Send + Sync + ?Sized, C: Send + Sync>(
        &'a mut R,
        &'a mut C,
    );

    impl<R: AsyncRead + Unpin + Send + Sync + ?Sized, C: Send + Sync> ContextWrappedReader<'_, R, C> {
        pub async fn decode_component<P: PacketComponent<C> + Sized>(
            &mut self,
        ) -> DraxResult<P::ComponentType> {
            P::decode(self.1, self.0).await
        }

        pub async fn decode_own_component<P: PacketComponent<C, ComponentType = P> + Sized>(
            &mut self,
        ) -> DraxResult<P> {
            P::decode(self.1, self.0).await
        }
    }

    pub trait WriterContext<'a, C: Send + Sync, T> {
        fn writer_context(&'a mut self, context: &'a mut C) -> T;
    }

    impl<'a, W: AsyncWrite + Unpin + Send + Sync + ?Sized, C: Send + Sync>
        WriterContext<'a, C, ContextWrappedWriter<'a, W, C>> for W
    {
        fn writer_context(&'a mut self, context: &'a mut C) -> ContextWrappedWriter<'a, Self, C> {
            ContextWrappedWriter(self, context)
        }
    }

    pub trait ReaderContext<'a, C: Send + Sync, T> {
        fn reader_context(&'a mut self, context: &'a mut C) -> T;
    }

    impl<'a, R: AsyncRead + Unpin + Send + Sync + ?Sized, C: Send + Sync>
        ReaderContext<'a, C, ContextWrappedReader<'a, R, C>> for R
    {
        fn reader_context(&'a mut self, context: &'a mut C) -> ContextWrappedReader<'a, Self, C> {
            ContextWrappedReader(self, context)
        }
    }
}
