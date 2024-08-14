use drax::error::DraxResult;
use drax::prelude::{DraxReadExt, DraxWriteExt, PacketComponent, Size, VarInt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

#[derive(Debug, Eq, PartialEq)]
struct Message {
    value: i32,
    value2: i32,
}

impl<C: Send + Sync> PacketComponent<C> for Message {
    type ComponentType = Self;

    async fn decode<A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &mut C,
        read: &mut A,
    ) -> DraxResult<Self::ComponentType> {
        let value = VarInt::decode(context, read).await?;
        let value2 = i32::decode(context, read).await?;
        Ok(Self { value, value2 })
    }

    async fn encode<A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &Self::ComponentType,
        context: &mut C,
        write: &mut A,
    ) -> DraxResult<()> {
        VarInt::encode(&component_ref.value, context, write).await?;
        i32::encode(&component_ref.value2, context, write).await
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> DraxResult<Size> {
        let mut size = Size::Constant(0);
        size = size + VarInt::size(&input.value, context)?;
        size = size + i32::size(&input.value2, context)?;
        Ok(size)
    }
}

#[tokio::main]
async fn main() -> DraxResult<()> {
    let distributed_value = (32, 64);

    let (waiter_tx, waiter_rx) = oneshot::channel();

    // Server
    let server_handle: JoinHandle<DraxResult<()>> = tokio::spawn(async move {
        let listener = TcpListener::bind("127.0.0.1:8000").await?;
        let _ = waiter_tx.send(());
        let (mut socket, _) = listener.accept().await?;

        let message: Message = socket.decode_own_component().await?;
        assert_eq!(message.value, distributed_value.0);
        assert_eq!(message.value2, distributed_value.1);
        socket.encode_own_component(&message).await?;

        println!("Server communication completed successfully.");
        Ok(())
    });

    let _ = waiter_rx.await;

    // Client
    let client_handle: JoinHandle<DraxResult<()>> = tokio::spawn(async move {
        let message = Message {
            value: distributed_value.0,
            value2: distributed_value.1,
        };

        let mut stream = TcpStream::connect("127.0.0.1:8000").await?;
        stream.encode_own_component(&message).await?;
        let back: Message = stream.decode_own_component().await?;
        assert_eq!(message, back);

        println!("Client communication completed successfully.");
        Ok(())
    });

    server_handle
        .await
        .expect("Failed to join server handle.")?;
    client_handle
        .await
        .expect("Failed to join client handle.")?;

    Ok(())
}
