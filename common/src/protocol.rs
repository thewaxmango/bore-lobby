use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// --- Client -> Server ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMsg {
    SetName { name: String },
    CreateRoom { game: String, password: Option<String> },
    JoinRoom { room_code: String, password: Option<String> },
    StartGame,
    GameAction { action: serde_json::Value },
    LeaveRoom,
    Ping,
}

// --- Server -> Client ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    Welcome {
        player_id: usize,
    },
    RoomCreated {
        room_code: String,
    },
    RoomJoined {
        room_code: String,
        players: Vec<String>,
        game: String,
        host: String,
    },
    PlayerJoined {
        name: String,
    },
    PlayerLeft {
        name: String,
    },
    HostChanged {
        name: String,
    },
    GameStarted {
        game: String,
    },
    GameState {
        state: serde_json::Value,
    },
    GameEvent {
        event: serde_json::Value,
    },
    GameOver {
        winner: String,
    },
    Error {
        message: String,
    },
}

// --- Framing: 4-byte length prefix + JSON ---

pub async fn write_msg<W: AsyncWriteExt + Unpin, M: Serialize>(
    writer: &mut W,
    msg: &M,
) -> io::Result<()> {
    let json = serde_json::to_vec(msg).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let len = json.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&json).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_msg<R: AsyncReadExt + Unpin, M: DeserializeOwned>(
    reader: &mut R,
) -> io::Result<M> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "message too large",
        ));
    }

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
