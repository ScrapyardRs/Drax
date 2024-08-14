use drax::error::DraxResult;
use drax::prelude::{DraxReadExt, DraxWriteExt, PacketComponent, Size};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub enum ServerboundIrcPacket {
    Identify(String),
    JoinChannel(String),
    SendMessage(String),
}

impl<C: Send + Sync> PacketComponent<C> for ServerboundIrcPacket {
    type ComponentType = Self;

    async fn decode<A: tokio::io::AsyncRead + Unpin + Send + Sync + ?Sized>(
        _: &mut C,
        read: &mut A,
    ) -> DraxResult<Self::ComponentType> {
        let packet_type = read.read_u8().await?;
        match packet_type {
            0 => Ok(ServerboundIrcPacket::Identify(
                read.decode_own_component().await?,
            )),
            1 => Ok(ServerboundIrcPacket::JoinChannel(
                read.decode_own_component().await?,
            )),
            2 => Ok(ServerboundIrcPacket::SendMessage(
                read.decode_own_component().await?,
            )),
            _ => Err(drax::error::TransportError::VarNumTooLarge),
        }
    }

    async fn encode<A: tokio::io::AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &Self::ComponentType,
        _: &mut C,
        write: &mut A,
    ) -> DraxResult<()> {
        match component_ref {
            ServerboundIrcPacket::Identify(value) => {
                write.write_u8(0).await?;
                write.encode_own_component(value).await?;
            }
            ServerboundIrcPacket::JoinChannel(value) => {
                write.write_u8(1).await?;
                write.encode_own_component(value).await?;
            }
            ServerboundIrcPacket::SendMessage(value) => {
                write.write_u8(2).await?;
                write.encode_own_component(value).await?;
            }
        }
        Ok(())
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> DraxResult<Size> {
        Ok(Size::Dynamic(1)
            + String::size(
                match input {
                    ServerboundIrcPacket::Identify(value) => value,
                    ServerboundIrcPacket::JoinChannel(value) => value,
                    ServerboundIrcPacket::SendMessage(value) => value,
                },
                context,
            )?)
    }
}

pub enum ClientboundIrcPacket {
    IdentityAccepted(String),
    IdentityRejected,
    JoinedChannel { other_users: Vec<String> },
    MessageReceived { sender: String, message: String },
}
