use crate::delegate::primitive::size_var_int;
use crate::prelude::{
    DraxReadExt, DraxResult, DraxWriteExt, PacketComponent, Size, TransportError,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const STRING_DEFAULT_CAP: i32 = 32767 * 4;

impl<C: Send + Sync> PacketComponent<C> for String {
    type ComponentType = Self;

    decode!(read {
        let len = read.read_var_int().await?;
        if len > STRING_DEFAULT_CAP {
            return TransportError::limit_exceeded(STRING_DEFAULT_CAP, len, "decoding string");
        }
        let mut buf = vec![0; len as usize];
        read.read_exact(&mut buf).await?;
        Ok(String::from_utf8(buf)?)
    });

    encode!(component_ref, write {
        let len = component_ref.len() as i32;
        if len > STRING_DEFAULT_CAP {
            return TransportError::limit_exceeded(STRING_DEFAULT_CAP, len, "encoding string");
        }

        write.write_var_int(len).await?;
        write.write_all(component_ref.as_bytes()).await?;
    });

    fn size(component_ref: &Self, _: &mut C) -> DraxResult<Size> {
        Ok(Size::Dynamic(
            component_ref.len() + size_var_int(component_ref.len() as i32),
        ))
    }
}

/// A delegate struct which constricts the size of a `String` to the given constant limit.
pub struct LimitedString<const N: i32>;

impl<C: Send + Sync, const N: i32> PacketComponent<C> for LimitedString<N> {
    type ComponentType = String;

    decode!(read {
        let string_size = read.read_var_int().await?;

        if string_size > N {
            return TransportError::limit_exceeded(N, string_size, "decoding string");
        }

        let mut buf = vec![0; string_size as usize];
        read.read_exact(&mut buf).await?;
        Ok(String::from_utf8(buf)?)
    });

    encode!(component_ref, write, context {
        let len = component_ref.len() as i32;

        if len > N {
            return TransportError::limit_exceeded(N, len, "encoding string");
        }

        String::encode(component_ref, context, write).await?;
    });

    fn size(input: &Self::ComponentType, context: &mut C) -> DraxResult<Size> {
        String::size(input, context)
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::{DraxReadExt, DraxResult, DraxWriteExt, LimitedString, TransportError};
    use std::assert_matches::assert_matches;
    use std::io::Cursor;

    #[tokio::test]
    pub async fn test_string_encoding() -> DraxResult<()> {
        let mut cursor = Cursor::new(vec![]);
        cursor
            .encode_component::<String>(&"test string".to_string())
            .await?;

        cursor.set_position(0);

        let back = cursor.decode_component::<String>().await?;

        assert_eq!(back, "test string");
        Ok(())
    }

    #[tokio::test]
    pub async fn test_limited_string_encoding() -> DraxResult<()> {
        let mut cursor = Cursor::new(vec![]);
        let error = cursor
            .encode_component::<LimitedString<10>>(&"test string".to_string())
            .await;

        assert_matches!(
            error,
            Err(TransportError::LimitExceeded(10, 11, "encoding string"))
        );
        Ok(())
    }

    #[tokio::test]
    pub async fn test_limited_string_decoding() -> DraxResult<()> {
        let mut cursor = Cursor::new(vec![]);
        cursor.write_var_int(11).await?;

        cursor.set_position(0);

        let error = cursor.decode_component::<LimitedString<10>>().await;

        assert_matches!(
            error,
            Err(TransportError::LimitExceeded(10, 11, "decoding string"))
        );
        Ok(())
    }
}
