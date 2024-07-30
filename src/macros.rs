use crate::prelude::{DraxWriteExt, PacketComponent, Size};
use crate::PinnedLivelyResult;
use std::collections::HashMap;
use tokio::io::{AsyncRead, AsyncWrite};
use uuid::Uuid;

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
    ($reader:ident, $context:ident => { $(let $decode_ident:ident;)* }) => {
        Box::pin(async move {
            $(
                let $decode_ident = crate::prelude::DraxReadExt::decode_own_component($reader, $context).await?;
            )*

            Ok(Self { $($decode_ident),* })
        })
    };
}

#[macro_export]
macro_rules! simple_size {
    ($context:ident, $size_ident:ident => { $($size:expr;)* }) => {
        {
            let mut $size_ident = $crate::prelude::Size::Constant(0);
            $(
                $size_ident = $size_ident + $crate::transport::packet::size_of_me(&$size, $context)?;
            )*
            Ok($size_ident)
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
            ) -> PinnedLivelyResult<'a, Self::ComponentType> {
                simple_decode!(read, context => {
                    $(
                        let $field_name;
                    )*
                })
            }

            fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
                component_ref: &'a Self::ComponentType,
                context: &'a mut C,
                write: &'a mut A,
            ) -> PinnedLivelyResult<'a, ()> {
                simple_encode!(write, context => {
                    $(
                        component_ref.$field_name;
                    )*
                })
            }

            fn size(input: &Self::ComponentType, context: &mut C) -> crate::prelude::Result<Size> {
                simple_size!(context, size => {
                    $(
                        input.$field_name;
                    )*
                })
            }
        }
    }
}
