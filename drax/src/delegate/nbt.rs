use crate::prelude::{DraxResult, NbtError, PacketComponent, Size};
use std::io::Cursor;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const COMPOUND_TAG_BIT: u8 = 10;

pub struct NbtAccounter {
    limit: u64,
    current: u64,
}

impl NbtAccounter {
    pub fn account_bytes(&mut self, bytes: u64) -> DraxResult<()> {
        if self.limit == 0 {
            return Ok(());
        }
        match self.current.checked_add(bytes) {
            Some(next) => {
                if next > self.limit {
                    return NbtError::tag_too_big(self.limit, next);
                }
                self.current = next;
                Ok(())
            }
            None => NbtError::accounter_overflow(),
        }
    }
}

macro_rules! define_tags {
    ($(
        $tag:ident $idx:literal {
            const type = $backing_ty:ty;
            fn size($size_ref_ident:ident) {
                $($sizer_tt:tt)*
            },
            fn write($writer:ident, $write_ref_ident:ident) {
                $($writer_tt:tt)*
            },
            fn read($reader:ident, $accounter:ident, $depth:ident) {
                $($reader_tt:tt)*
            },
        }
    ),*) => {
        $(
            pub struct $tag;
        )*

        #[derive(Debug, PartialEq, Clone)]
        pub enum Tag {
            $(
                $tag($backing_ty),
            )*
        }

        impl Tag {
            pub fn get_tag_bit(&self) -> u8 {
                match self {
                    $(
                    Tag::$tag(_) => $idx,
                    )*
                }
            }
        }

        pub async fn load_tag<R: ::tokio::io::AsyncRead + Unpin + Send + Sync + ?Sized>(
            read: &mut R,
            bit: u8,
            depth: i32,
            accounter: &mut $crate::delegate::nbt::NbtAccounter
        ) -> DraxResult<Tag> {
            match bit {
                $(
                $idx => {
                    let $reader = read;
                    let $accounter = accounter;
                    let $depth = depth;
                    $($reader_tt)*
                }
                )*
                bit => NbtError::invalid_tag_bit(bit)
            }
        }

        pub async fn write_tag<W: ::tokio::io::AsyncWrite + Unpin + Send + Sync + ?Sized>(
            write: &mut W,
            tag: &Tag
        ) -> DraxResult<()> {
            match tag {
                $(
                Tag::$tag($write_ref_ident) => {
                    let $writer = write;
                    $($writer_tt)*
                }
                )*
            }
        }

        pub fn size_tag(tag: &Tag) -> DraxResult<usize> {
            match tag {
                $(
                Tag::$tag($size_ref_ident) => {
                    $($sizer_tt)*
                }
                )*
            }
        }
    };
}

async fn read_string<R: AsyncRead + Unpin + Send + Sync + ?Sized>(
    read: &mut R,
    accounter: &mut NbtAccounter,
) -> DraxResult<String> {
    let len = read.read_u16().await?;
    let mut bytes = vec![0u8; len as usize];
    read.read_exact(&mut bytes).await?;
    let string = cesu8::from_java_cesu8(&bytes)
        .map_err(NbtError::from)?
        .to_string();
    accounter.account_bytes(string.len() as u64)?;
    Ok(string)
}

async fn write_string<W: AsyncWrite + Unpin + Send + Sync + ?Sized>(
    write: &mut W,
    reference: &str,
) -> DraxResult<()> {
    let cesu_8 = &cesu8::to_java_cesu8(reference);
    write.write_u16(cesu_8.len() as u16).await?;
    write.write_all(cesu_8).await?;
    Ok(())
}

fn size_string(reference: &str) -> DraxResult<usize> {
    Ok(2 + cesu8::to_java_cesu8(reference).len())
}

define_tags! {
    TagEnd 0 {
        const type = ();
        fn size(_s) {
            Ok(0)
        },
        fn write(_w, _s) {
            Ok(())
        },
        fn read(_r, accounter, _d) {
            accounter.account_bytes(8)?;
            Ok(Tag::TagEnd(()))
        },
    },
    TagByte 1 {
        const type = u8;
        fn size(_reference) {
            Ok(1)
        },
        fn write(writer, reference) {
            writer.write_u8(*reference).await?;
            Ok(())
        },

        fn read(reader, accounter, _d) {
            accounter.account_bytes(9)?;
            Ok(Tag::TagByte(reader.read_u8().await?))
        },
    },
    TagShort 2 {
        const type = u16;
        fn size(_reference) {
            Ok(2)
        },
        fn write(writer, reference) {
            writer.write_u16(*reference).await?;
            Ok(())
        },
        fn read(reader, accounter, _d) {
            accounter.account_bytes(10)?;
            Ok(Tag::TagShort(reader.read_u16().await?))
        },
    },
    TagInt 3 {
        const type = i32;
        fn size(_reference) {
            Ok(4)
        },
        fn write(writer, reference) {
            writer.write_i32(*reference).await?;
            Ok(())
        },
        fn read(reader, accounter, _d) {
            accounter.account_bytes(12)?;
            Ok(Tag::TagInt(reader.read_i32().await?))
        },
    },
    TagLong 4 {
        const type = i64;
        fn size(_reference) {
            Ok(8)
        },
        fn write(writer, reference) {
            writer.write_i64(*reference).await?;
            Ok(())
        },
        fn read(reader, accounter, _d) {
            accounter.account_bytes(16)?;
            Ok(Tag::TagLong(reader.read_i64().await?))
        },
    },
    TagFloat 5 {
        const type = f32;
        fn size(_reference) {
            Ok(4)
        },
        fn write(writer, reference) {
            writer.write_f32(*reference).await?;
            Ok(())
        },
        fn read(reader, accounter, _d) {
            accounter.account_bytes(12)?;
            Ok(Tag::TagFloat(reader.read_f32().await?))
        },
    },
    TagDouble 6 {
        const type = f64;
        fn size(_reference) {
            Ok(8)
        },
        fn write(writer, reference) {
            writer.write_f64(*reference).await?;
            Ok(())
        },
        fn read(reader, accounter, _d) {
            accounter.account_bytes(16)?;
            Ok(Tag::TagDouble(reader.read_f64().await?))
        },
    },
    TagByteArray 7 {
        const type = Vec<u8>;
        fn size(reference) {
            Ok(4 + reference.len())
        },
        fn write(writer, reference) {
            writer.write_i32(reference.len() as i32).await?;
            writer.write_all(reference).await?;
            Ok(())
        },
        fn read(reader, accounter, _d) {
            accounter.account_bytes(24)?;
            let len = reader.read_i32().await?;
            accounter.account_bytes(len as u64)?;
            let mut bytes = vec![0u8; len as usize];
            reader.read_exact(&mut bytes).await?;
            Ok(Tag::TagByteArray(bytes))
        },
    },
    TagString 8 {
        const type = String;
        fn size(reference) {
            size_string(reference)
        },
        fn write(writer, reference) {
            write_string(writer, reference).await
        },
        fn read(reader, accounter, _d) {
            accounter.account_bytes(36)?;
            Ok(Tag::TagString(read_string(reader, accounter).await?))
        },
    },
    TagList 9 {
        const type = (u8, Vec<Tag>);
        fn size(reference) {
            Ok(5 + {
                let mut size = 0;
                for item in &reference.1 {
                    size += size_tag(item)?;
                }
                size
            })
        },
        fn write(writer, reference) {
            writer.write_u8(reference.0).await?;
            writer.write_i32(reference.1.len() as i32).await?;
            for tag in &reference.1 {
                Box::pin(write_tag(writer, tag)).await?;
            }
            Ok(())
        },
        fn read(reader, accounter, depth) {
            accounter.account_bytes(37)?;
            if depth > 512 {
                return NbtError::complex_tag();
            }
            let tag_byte = reader.read_u8().await?;
            let length = reader.read_i32().await?;
            accounter.account_bytes((4 * length) as u64)?;
            let mut v = Vec::with_capacity(length as usize);
            for _ in 0..length {
                v.push(Box::pin(load_tag(reader, tag_byte, depth + 1, accounter)).await?);
            }
            Ok(Tag::TagList((tag_byte, v)))
        },
    },
    CompoundTag 10 {
        const type = Vec<(String, Tag)>;
        fn size(reference) {
            if reference.is_empty() {
                return Ok(1);
            }

            let mut size = 0;
            for (key, value) in reference {
                size += size_string(key)? + 1;
                size += size_tag(value)?;
            }
            Ok(size + 1)
        },
        fn write(writer, reference) {
            if reference.is_empty() {
                writer.write_u8(0).await?;
                return Ok(());
            }
            for (key, value) in reference {
                writer.write_u8(value.get_tag_bit()).await?;
                write_string(writer, key).await?;
                Box::pin(write_tag(writer, value)).await?;
            }
            writer.write_u8(0).await?;
            Ok(())
        },
        fn read(reader, accounter, depth) {
            accounter.account_bytes(48)?;
            if depth > 512 {
                return NbtError::complex_tag();
            }
            let mut map = Vec::new();
            loop {
                let tag_byte = reader.read_u8().await?;
                if tag_byte == 0 {
                    break;
                }
                accounter.account_bytes(28)?;
                let key = read_string(reader, accounter).await?;
                let data = Box::pin(load_tag(reader, tag_byte, depth + 1, accounter)).await?;
                map.push((key, data));
                accounter.account_bytes(36)?;
            }
            Ok(Tag::CompoundTag(map))
        },
    },
    TagIntArray 11 {
        const type = Vec<i32>;
        fn size(reference) {
            Ok(4 + (4 * reference.len()))
        },
        fn write(writer, reference) {
            writer.write_i32(reference.len() as i32).await?;
            for item in reference {
                writer.write_i32(*item).await?;
            }
            Ok(())
        },
        fn read(reader, accounter, _d) {
            accounter.account_bytes(24)?;
            let len = reader.read_i32().await?;
            accounter.account_bytes((4 * len) as u64)?;
            let mut i_arr = Vec::with_capacity(len as usize);
            for _ in 0..len {
                i_arr.push(reader.read_i32().await?);
            }
            Ok(Tag::TagIntArray(i_arr))
        },
    },
    TagLongArray 12 {
        const type = Vec<i64>;
        fn size(reference) {
            Ok(4 + (8 * reference.len()))
        },
        fn write(writer, reference) {
            writer.write_i32(reference.len() as i32).await?;
            for item in reference {
                writer.write_i64(*item).await?;
            }
            Ok(())
        },
        fn read(reader, accounter, _d) {
            accounter.account_bytes(24)?;
            let len = reader.read_i32().await?;
            accounter.account_bytes((8 * len) as u64)?;
            let mut i_arr = Vec::with_capacity(len as usize);
            for _ in 0..len {
                i_arr.push(reader.read_i64().await?);
            }
            Ok(Tag::TagLongArray(i_arr))
        },
    }
}

#[cfg(test)]
mod test {
    use crate::delegate::nbt::{load_tag, read_string, write_string, write_tag, NbtAccounter, Tag};
    use crate::prelude::DraxResult;
    use std::io::Cursor;

    pub async fn __test_io(value: Tag) -> DraxResult<()> {
        let mut cursor = Cursor::new(vec![]);
        write_tag(&mut cursor, &value).await?;
        let inner = cursor.into_inner();
        let mut cursor = Cursor::new(inner);
        let tag = load_tag(
            &mut cursor,
            value.get_tag_bit(),
            0,
            &mut NbtAccounter {
                limit: 0,
                current: 0,
            },
        )
        .await?;
        assert_eq!(tag, value);
        Ok(())
    }

    macro_rules! test_io {
        ($($test_name:ident, $value:expr),*) => {$(
            #[tokio::test]
            pub async fn $test_name() -> DraxResult<()> {
                __test_io($value).await
            }
        )*};
    }

    macro_rules! create_map {
        ($($key:expr, $value:expr),*) => {
            vec![$(($key, $value)),*]
        }
    }

    test_io! {
        test_tag_end, Tag::TagEnd(()),
        test_tag_byte, Tag::TagByte(10),
        test_tag_short, Tag::TagShort(20),
        test_tag_int, Tag::TagInt(30),
        test_tag_long, Tag::TagLong(40),
        test_tag_float, Tag::TagFloat(12.30),
        test_tag_double, Tag::TagDouble(20.30),
        test_tag_byte_array, Tag::TagByteArray(vec![10, 20, 0, 5]),
        test_tag_string, Tag::TagString("test string".to_string()),
        test_tag_list, Tag::TagList((2, vec![Tag::TagShort(10u16), Tag::TagShort(20), Tag::TagShort(9), Tag::TagShort(15)])),
        test_tag_compound, Tag::CompoundTag(create_map!("abc".to_string(), Tag::TagShort(15), "def".to_string(), Tag::TagFloat(12.30))),
        test_tag_int_array, Tag::TagIntArray(vec![30, 23, 123, 955]),
        test_tag_long_array, Tag::TagLongArray(vec![321423, 24312, 123123, 12312])
    }

    #[tokio::test]
    pub async fn test_string_read_write_persistence() -> DraxResult<()> {
        let ref_string = "Example String".to_string();
        let mut cursor = Cursor::new(vec![]);
        write_string(&mut cursor, &ref_string).await?;
        let mut cursor = Cursor::new(cursor.into_inner());
        let back = read_string(
            &mut cursor,
            &mut NbtAccounter {
                limit: 0,
                current: 0,
            },
        )
        .await?;
        assert_eq!(ref_string, back);
        Ok(())
    }
}

/// A macro which creates an `Tag::CompoundTag` from a set of tag-like values.
/// ```rust
/// # use drax::prelude::*;
/// # use std::io::Cursor;
/// use drax::tag;
/// # #[tokio::test]
/// # async fn test() -> DraxResult<()> {
/// let example = Some(tag!(
///     example: Tag::TagByte(10),
///     example2: Tag::TagShort(20),
///     example3: Tag::TagInt(30)
/// ));
/// let mut cursor = Cursor::new(vec![]);
/// cursor.encode_component::<EnsuredCompoundTag>(&example).await?;
/// cursor.set_position(0);
/// let back = cursor.decode_component::<EnsuredCompoundTag>().await?;
/// assert_eq!(example, back);
/// # Ok(()) }
/// ```
#[cfg_attr(feature = "nbt", macro_export)]
macro_rules! tag {
    ($(
        $tag_field_name:ident: $tag_value:expr
    ),*) => {
        {
            let mut data = vec![];
            $(
            data.push((stringify!($tag_field_name), $tag_value));
            )*
            $crate::delegate::nbt::Tag::compound_tag(data)
        }
    }
}

impl Tag {
    pub fn string<S: Into<String>>(into: S) -> Tag {
        Tag::TagString(into.into())
    }

    pub fn compound_tag<S: Into<String>>(data: Vec<(S, Tag)>) -> Self {
        Tag::CompoundTag(data.into_iter().map(|(x, y)| (x.into(), y)).collect())
    }
}

pub struct EnsuredCompoundTag<const LIMIT: u64 = 0>;

impl<const LIMIT: u64, C: Send + Sync> PacketComponent<C> for EnsuredCompoundTag<LIMIT> {
    type ComponentType = Option<Tag>;

    decode!(read {
        let b = read.read_u8().await?;
        if b == 0 {
            return Ok(None);
        }
        if b != 10 {
            return NbtError::invalid_tag_bit(b);
        }
        let mut accounter = NbtAccounter {
            limit: LIMIT,
            current: 0,
        };
        let _ = read_string(read, &mut accounter).await?;
        let tag = load_tag(read, b, 0, &mut accounter).await?;
        Ok(Some(tag))
    });

    encode!(component_ref, write {
        let mut buffer = Cursor::new(Vec::with_capacity(
            match Self::size(component_ref, &mut ())? {
                Size::Dynamic(x) | Size::Constant(x) => x,
            },
        ));
        match component_ref {
            Some(tag) => {
                buffer.write_u8(10).await?;
                write_string(&mut buffer, "").await?;
                write_tag(&mut buffer, tag).await?;
                let inner = buffer.into_inner();
                write.write_all(&inner).await?;
            }
            None => {
                write.write_u8(0).await?;
            }
        }
    });

    fn size(input: &Self::ComponentType, _: &mut C) -> DraxResult<Size> {
        match input {
            Some(tag) => {
                let dynamic_size = Size::Dynamic(3); // short 0 for str + byte tag
                Ok(dynamic_size + size_tag(tag)?)
            }
            None => Ok(Size::Constant(1)),
        }
    }
}
