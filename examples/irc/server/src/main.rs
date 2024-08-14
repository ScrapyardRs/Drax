use drax::prelude::{DraxReadExt, DraxResult};
use irc_common::ServerboundIrcPacket;
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> DraxResult<()> {
    let listener = TcpListener::bind("127.0.0.1:6667").await?;

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("Client connected from {addr}.");
        tokio::spawn(accept_client(socket));
    }
}

pub async fn accept_client(mut socket: TcpStream) -> DraxResult<()> {
    while let Ok(packet) = socket.decode_own_component::<ServerboundIrcPacket>().await? {}

    Ok(())
}
