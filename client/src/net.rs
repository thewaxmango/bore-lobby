use socket_lobby_common::protocol::{read_msg, write_msg, ClientMsg, ServerMsg};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct Connection {
    writer: OwnedWriteHalf,
    pub rx: mpsc::UnboundedReceiver<ServerMsg>,
    reader_handle: JoinHandle<()>,
}

impl Connection {
    pub async fn connect(addr: &str) -> std::io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (reader, writer) = stream.into_split();
        let (tx, rx) = mpsc::unbounded_channel();

        let reader_handle = tokio::spawn(reader_task(reader, tx));

        Ok(Self { writer, rx, reader_handle })
    }

    pub async fn send(&mut self, msg: ClientMsg) -> std::io::Result<()> {
        write_msg(&mut self.writer, &msg).await
    }

    pub async fn shutdown(self) {
        drop(self.writer);
        let _ = self.reader_handle.await;
    }
}

async fn reader_task(mut reader: OwnedReadHalf, tx: mpsc::UnboundedSender<ServerMsg>) {
    loop {
        match read_msg::<_, ServerMsg>(&mut reader).await {
            Ok(msg) => {
                if tx.send(msg).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}
