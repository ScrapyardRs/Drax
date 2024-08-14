use std::sync::Arc;
use crate::prelude::{DraxResult, PacketComponent, Size};

macro_rules! impl_deref_component {
    ($impl_ident:ident<$t_ty:ident>) => {
        impl<$t_ty, C: Send + Sync> PacketComponent<C> for $impl_ident<$t_ty>
        where
            $t_ty: PacketComponent<C>,
        {
            type ComponentType = $impl_ident<$t_ty::ComponentType>;

            decode!(read, context {
                let component = T::decode(context, read).await?;
                Ok(<$impl_ident<$t_ty::ComponentType>>::new(component))
            });

            encode!(component_ref, write, context {
                <$t_ty as PacketComponent<C>>::encode(component_ref.as_ref(), context, write).await?;
            });

            fn size(input: &Self::ComponentType, context: &mut C) -> DraxResult<Size> {
                <$t_ty as PacketComponent<C>>::size(input.as_ref(), context)
            }
        }
    };
}

impl_deref_component!(Box<T>);
impl_deref_component!(Arc<T>);
