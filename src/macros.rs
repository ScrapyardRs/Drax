#[macro_export]
macro_rules! simple_encode {
    ($writer:ident, $context:ident => { $($encode:expr;)* }) => {
        Box::pin(async move {
            $(
                $crate::prelude::DraxWriteExt::encode_own_component($writer, $context, &$encode).await?;
            )*
            Ok(())
        })
    };
}

#[macro_export]
macro_rules! simple_decode {
    (@raw $reader:ident, $context:ident => { $(let $decode_ident:ident$(: $decode_ty:ty)?;)* }) => {
        $(
            let $decode_ident$(: $decode_ty)? = $crate::prelude::DraxReadExt::decode_own_component($reader, $context).await?;
        )*
    };
    ($reader:ident, $context:ident => { $(let $decode_ident:ident;)* }) => {
        Box::pin(async move {
            $crate::simple_decode!(@raw $reader, $context => { $(let $decode_ident;)* });

            Ok(Self { $($decode_ident),* })
        })
    };
}

#[macro_export]
macro_rules! simple_size {
    ($context:ident => { $($size:expr;)* }) => {
        {
            let mut size = $crate::prelude::Size::Constant(0);
            $(
                size = size + $crate::transport::packet::size_of_me(&$size, $context)?;
            )*
            Ok(size)
        }
    };
}

#[macro_export]
macro_rules! simple_packet_impl {
    ($struct_name:ident => { $($field_name:ident),* }) => {
        impl<C: Send + Sync> PacketComponent<C> for $struct_name {
            type ComponentType = $struct_name;

            fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
                context: &'a mut C,
                read: &'a mut A,
            ) -> $crate::PinnedLivelyResult<'a, Self::ComponentType> {
                $crate::simple_decode!(read, context => {
                    $(
                        let $field_name;
                    )*
                })
            }

            fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
                component_ref: &'a Self::ComponentType,
                context: &'a mut C,
                write: &'a mut A,
            ) -> $crate::PinnedLivelyResult<'a, ()> {
                $crate::simple_encode!(write, context => {
                    $(
                        component_ref.$field_name;
                    )*
                })
            }

            fn size(input: &Self::ComponentType, context: &mut C) -> $crate::prelude::Result<Size> {
                $crate::simple_size!(context, size => {
                    $(
                        input.$field_name;
                    )*
                })
            }
        }
    }
}
