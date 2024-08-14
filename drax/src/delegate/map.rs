use crate::prelude::{
    DraxReadExt, DraxResult, DraxWriteExt, PacketComponent, Size, TransportError, VarInt,
};
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

impl<C: Send + Sync, K: PacketComponent<C>, V: PacketComponent<C>> PacketComponent<C>
    for HashMap<K, V>
where
    K::ComponentType: Eq + Hash,
{
    type ComponentType = HashMap<K::ComponentType, V::ComponentType>;

    decode!(read, context {
        let len = read.read_var_int().await?;
        let mut map = HashMap::with_capacity(len as usize);
        for _ in 0..len {
            map.insert(
                K::decode(context, read).await?,
                V::decode(context, read).await?,
            );
        }
        Ok(map)
    });

    encode!(component_ref, write, context {
        write.write_var_int(component_ref.len() as i32).await?;
        for (k, v) in component_ref {
            K::encode(k, context, write).await?;
            V::encode(v, context, write).await?;
        }
    });

    fn size(component_ref: &Self::ComponentType, context: &mut C) -> DraxResult<Size> {
        let mut size = Size::Constant(0);
        size = size + <VarInt as PacketComponent<C>>::size(&(component_ref.len() as i32), context)?;
        for (k, v) in component_ref.iter() {
            size = size + <K as PacketComponent<C>>::size(k, context)?;
            size = size + <V as PacketComponent<C>>::size(v, context)?;
        }
        Ok(size)
    }
}

/// A delegate struct which limits the size of a `HashMap` when encoding/decoding to the
/// given constant limit.
pub struct LimitedMap<K, V, const N: usize>(PhantomData<(K, V)>);

impl<C: Send + Sync, K: PacketComponent<C>, V: PacketComponent<C>, const N: usize>
    PacketComponent<C> for LimitedMap<K, V, N>
where
    K::ComponentType: Eq + Hash,
{
    type ComponentType = HashMap<K::ComponentType, V::ComponentType>;

    decode!(read, context {
        let map_size = read.read_var_int().await?;
        let lim = N as i32;
        if map_size > lim {
            return TransportError::limit_exceeded(lim, map_size, "decoding map");
        }

        let mut map = HashMap::with_capacity(map_size as usize);
        for _ in 0..map_size {
            map.insert(
                K::decode(context, read).await?,
                V::decode(context, read).await?,
            );
        }
        Ok(map)
    });

    encode!(component_ref, write, context {
        let len = component_ref.len() as i32;
        let lim = N as i32;

        if len > lim {
            return TransportError::limit_exceeded(lim, len, "encoding map");
        }

        HashMap::<K, V>::encode(component_ref, context, write).await?;
    });

    fn size(component_ref: &Self::ComponentType, context: &mut C) -> DraxResult<Size> {
        HashMap::<K, V>::size(component_ref, context)
    }
}

#[cfg(test)]
mod test {
    use crate::delegate::map::LimitedMap;
    use crate::prelude::{DraxReadExt, DraxResult, DraxWriteExt, TransportError};
    use std::assert_matches::assert_matches;
    use std::collections::HashMap;
    use std::io::Cursor;

    #[tokio::test]
    pub async fn test_int_map_encoding() -> DraxResult<()> {
        let mut cursor = Cursor::new(vec![]);

        let in_map = HashMap::from([(10, 20), (30, 40)]);

        cursor.encode_own_component(&in_map).await?;

        cursor.set_position(0);

        let out_map = cursor.decode_own_component::<HashMap<_, _>>().await?;

        assert_eq!(out_map, in_map);
        Ok(())
    }

    #[tokio::test]
    pub async fn test_string_map_encoding() -> DraxResult<()> {
        let mut cursor = Cursor::new(vec![]);

        let in_map = HashMap::from([("example1".to_string(), 20), ("example2".to_string(), 40)]);

        cursor.encode_own_component(&in_map).await?;

        cursor.set_position(0);

        let out_map = cursor.decode_own_component::<HashMap<_, _>>().await?;

        assert_eq!(out_map, in_map);

        Ok(())
    }

    #[tokio::test]
    pub async fn test_limited_map_encoding_failure() -> DraxResult<()> {
        let mut cursor = Cursor::new(vec![]);
        cursor.write_var_int(3).await?;

        cursor.set_position(0);

        let error = cursor.decode_component::<LimitedMap<i32, i32, 2>>().await;

        assert_matches!(
            error,
            Err(TransportError::LimitExceeded(2, 3, "decoding map"))
        );
        Ok(())
    }
}
