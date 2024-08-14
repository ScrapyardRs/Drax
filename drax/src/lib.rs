#![cfg_attr(feature = "slices", feature(maybe_uninit_uninit_array))]
#![cfg_attr(test, feature(assert_matches))]
#![allow(async_fn_in_trait)]

//! # Drax
//!
//! Drax is a library which supports framed packet reading and writing.
//! Drax itself is not an implementation of any protocol but instead a framework to build protocols
//! on top of. <br />
//! <br />
//! This framework should be able to provide all the tooling necessary for building an entire server
//! and client stack. The framework will attempt to keep most types generic and provide no default
//! implementations other than the low-level t1 layer. <br />
//! <br />
//! Drax will attempt to provide a low-overhead SDK for building out serialization and
//! deserialization for packets. These packets can be composed by bytes directly to reduce the
//! amount of allocations and copying required. While the bytes are drained from the source they're
//! used to build out the correlating types. <br />
//! <br />
//! Drax does not provide any backwards compatibility mechanisms or guarantees. The protocol assumes
//! both the client and server are running the same version of the protocol. Providing backwards
//! compatibility mechanisms often requires a lot of workarounds and creates turbulence in the
//! actual protocol implementation.

/// Exposes simple macros used for deriving packet component implementations.
pub mod macros;

/// Provides all the types and traits necessary for building out a transport layer.
pub mod transport;

/// Provides error types for the transport layer.
pub mod error;

/// Provides re-exports of common types for macros.
pub mod prelude;

/// Provides packet component implementations for common types.
pub mod delegate {
    macro_rules! decode {
        ($reader:ident $exec:expr) => {
            decode!($reader, _ctx $exec);
        };
        ($reader:ident, $ctx:ident $exec:expr) => {
            async fn decode<A: $crate::prelude::AsyncRead + Unpin + Send + Sync + ?Sized>(
                $ctx: &mut C,
                $reader: &mut A,
            ) -> $crate::prelude::DraxResult<Self::ComponentType> {
                $exec
            }
        };
    }

    macro_rules! encode {
        ($component:ident, $writer:ident $($exec:expr)?) => {
            encode!($component, $writer, _ctx $($exec)?);
        };
        ($component:ident, $writer:ident, $ctx:ident $($exec:expr)?) => {
            #[allow(redundant_semicolons)]
            async fn encode<A: $crate::prelude::AsyncWrite + Unpin + Send + Sync + ?Sized>(
                $component: &Self::ComponentType,
                $ctx: &mut C,
                $writer: &mut A,
            ) -> $crate::prelude::DraxResult<()> {
                $($exec)?;
                Ok(())
            }
        };
    }

    /// Provides packet component implementations for `HashMap<K, V>`.
    pub mod map;

    /// Provides packet component implementations for `Option<T>`.
    pub mod option;

    /// Provides packet component implementations for primitive numeric types and `Uuid`.
    pub mod primitive;

    /// Provides packet component implementations for `serde::Serialize` and `serde::Deserialize` values.
    #[cfg(feature = "serde")]
    pub mod serde_json;

    /// Provides packet component implementations for `String`.
    pub mod string;

    /// Provides packet component implementations for `Vec<T>` and `[T; N]`.
    pub mod vec;

    /// NBT is a tree data structure used and defined in Minecraft's protocol. This is extended to this
    /// crate to allow for easy low-level serialization and deserialization of NBT data. This entire
    /// module can be omitted by disabling the `nbt` feature.
    ///
    /// <div class="warning">This module is not supported. PRs are welcome to improve the implementation but
    /// as it stands the implementation is considered "done" and likely will not be updated.</div>
    ///
    /// The latest minecraft protocol tested against this code is `1.20.1`.
    #[cfg(feature = "nbt")]
    pub mod nbt;

    /// Contains implementations for reference types such as `Box<T>` and `Arc<T>`.
    pub mod referenced;
}
