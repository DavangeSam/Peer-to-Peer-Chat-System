use crate::Bing2BingCommand;
use crate::{cmd::Register, util::TtlMap, Bing2BingError, Bing2BingFrame, Connection};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tracing::{debug, instrument, trace};

#[derive(Debug)]
pub struct Tracker {
    listener: TcpListener,
}

impl Tracker {
    pub async fn new(bind_address: &str, port: &str) -> Result<Self, Bing2BingError> {
        Ok(Self {
            listener: TcpListener::bind(format!("{}:{}", bind_address, port)).await?,
        })
    }

    #[instrument(level = "trace")]
    pub async fn listen(&self) -> Result<(), Bing2BingError> {
        let peers = TtlMap::new();

        loop {
            let (stream, addr) = self.listener.accept().await?;

            let peers = peers.clone();

            tokio::spawn(async move {
                debug!("Accepted connection from {:?}", addr);

                Tracker::handle_connection(peers, stream, addr)
                    .await
                    .unwrap();
            });
        }
    }

    fn process_frame(frame: Bing2BingFrame) -> Result<Register, Bing2BingError> {
        let command = Bing2BingCommand::from_frame(frame)?;

        match command {
            Bing2BingCommand::Register(cmd) => Ok(cmd),
            _ => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "RECEIVED NON REGISTER COMMAND",
            ))),
        }
    }

    #[instrument(level = "trace")]
    pub(crate) async fn handle_connection(
        peers: TtlMap<SocketAddr>,
        stream: TcpStream,
        addr: SocketAddr,
    ) -> Result<(), Bing2BingError> {
        let mut connection = Connection::new(stream).await;

        // not entirely sure if this is the best way to handle things, but we are going to force
        // reception of at least one register command before we move forward
        // this will allow us to "fix" a node name, ip, and port

        // really should deal with the posibility of a None here instead of
        // just `unwrap()`ing.
        let frame = connection.read_frame().await?.unwrap();
        trace!("Received {:?} from {}", frame, addr);

        let cmd = Tracker::process_frame(frame)?;

        let peer_name = cmd.peer_name();
        cmd.apply(&peers, &mut connection).await?;

        while let Ok(Some(frame)) = connection.read_frame().await {
            trace!("Received {:?} from {}", frame, addr);

            let command = Bing2BingCommand::from_frame(frame)?;
            println!("command received: {:?}", command);
            trace!(?command);

            match command {
                Bing2BingCommand::Register(cmd) => cmd.apply(&peers, &mut connection).await?,
                _ => trace!("Received unimplemented command! {:?}", command),
            }
        }

        // we need to remove this peer from our known peer list.
        trace!("Removing {} from known peer list", peer_name);
        let mut peers = peers;
        peers.remove(&peer_name);

        Ok(())
    }
}
