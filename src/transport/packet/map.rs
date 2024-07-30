use crate::prelude::{DraxReadExt, DraxWriteExt, PacketComponent, Size};
use crate::transport::packet::primitive::VarInt;
use crate::PinnedLivelyResult;
use std::collections::HashMap;
use std::hash::Hash;

impl<C: Send + Sync, K: PacketComponent<C>, V: PacketComponent<C>> PacketComponent<C>
    for HashMap<K, V>
where
    K::ComponentType: Eq + Hash,
{
    type ComponentType = HashMap<K::ComponentType, V::ComponentType>;

    fn decode<'a, A: tokio::io::AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let len = read.read_var_int().await?;
            let mut map = HashMap::with_capacity(len as usize);
            for _ in 0..len {
                map.insert(
                    K::decode(context, read).await?,
                    V::decode(context, read).await?,
                );
            }
            Ok(map)
        })
    }

    fn encode<'a, A: tokio::io::AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            write.write_var_int(component_ref.len() as i32).await?;
            for (k, v) in component_ref {
                K::encode(k, context, write).await?;
                V::encode(v, context, write).await?;
            }
            Ok(())
        })
    }

    fn size(component_ref: &Self::ComponentType, context: &mut C) -> crate::prelude::Result<Size> {
        let mut size = Size::Constant(0);
        size = size + <VarInt as PacketComponent<C>>::size(&(component_ref.len() as i32), context)?;
        for (k, v) in component_ref.iter() {
            size = size + <K as PacketComponent<C>>::size(k, context)?;
            size = size + <V as PacketComponent<C>>::size(v, context)?;
        }
        Ok(size)
    }
}
