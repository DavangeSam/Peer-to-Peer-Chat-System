# bing2bing - Peer-to-Peer Chat System

## Project Description

**bing2bing** is a sophisticated peer-to-peer chat system that provides direct and decentralized communication between users. It leverages Rust's asynchronous programming for efficient connection handling and employs a JSON-based, frame-oriented protocol to ensure structured and organized data transmission.

## Dependencies

### CLI Dependencies

The CLI client for bing2bing relies on several Rust crates:

- `tokio` (with features `full` and `tracing`)
- `tracing`, `tracing-log` (with `log-tracer` feature), `tracing-subscriber` (with `env-filter` feature)
- `chrono` (with `serde` feature)
- `crossterm`
- `clap` (with features `derive`, `env`, `unicode`, `wrap_help`)
- `unicode-width`
- `log`
- `bing2bing-core` (local dependency)
- `dotenvy`
- `ratatui`
- `tui-logger`

### Core Dependencies

The core library of bing2bing, which handles the networking and protocol logic, includes these dependencies:

- `serde` and `serde_json` (with `derive` feature)
- `tokio-serde` (with features `cbor`, `messagepack`, `json`, `bincode`)
- `tokio` (with features `full` and `tracing`)
- `tokio-util` (with `full` feature)
- `futures` (with `thread-pool` feature)
- `tracing`, `tracing-subscriber` (with `env-filter` feature)
- `bytes`
- `clap` (with features `derive`, `env`, `unicode`, `wrap_help`)
- `rand`
- `async-channel`
- `dotenvy`
- `async-trait`


## Usage

This section provides instructions for running the various components of the bing2bing chat system.

### Tracker

To initiate the tracker, which facilitates peer discovery and network management:

```bash
cargo run -p bing2bing-core --bin tracker -- --host 0.0.0.0 --port 3001
```

**Note**: The `--host` parameter specifies the IP address for the tracker. Setting it to `0.0.0.0` allows it to listen on all IP addresses that the machine responds to.

### Client

Start a client to participate in the chat network. The client setup offers both simple and advanced user interfaces.

#### Running the Client

To launch the client:

```bash
cargo run -p bing2bing -- --host <your-ip> --port <your-port> --tracker-host <tracker-ip> --tracker-port <tracker-port> --name <your-name> [--simple]
```

#### Command Line Arguments

1. `--host`: The IP address your client will listen on. Note that `0.0.0.0` cannot be used as it directly affects outgoing protocol messages.
2. `--port`: The port your client will use, which must be unique on the host machine.
3. `--tracker-host`: The IP address of the tracker, which helps your client connect and bootstrap into the network.
4. `--tracker-port`: The port number of the tracker.
5. `--name`: The identifier for your peer in the network.
6. `--simple`: If set, the client will start in a simple mode. To use the fancy TUI, omit the --simple flag.

## Fancy TUI Features

The advanced Fancy TUI offers enriched interaction with several intuitive features:

- **Edit**: Press `e` to compose messages, and `enter` to send them.
- **Paste**: Press `ctrl + v` to paste copied text.
- **Erase**: Press `backspace` to delete characters.
- **Navigation**: Press `esc` to exit edit mode; `l` and `h` to switch between `Log` and `Home` tabs.
- **Quit**: Type `q` to exit the application.

### Supported Commands

- **Ping**  
Tests latency between your peer and another specified peer.  

```bash
/ping <peer_name>
```

- **Pong**  
Automatically sent as a response to a Ping command.

- **Say**  
Propagates normal chat messages throughout the network.  
Note: If no specific command is prefixed to a typed message or data, it is treated as a `/say` command by default.

```bash
/say <message>
```

- **Whisper**  
Sends a direct message to a specified peer.  

```bash
/whisper <peer_name> <message>
```

- **Broadcast**  
Sends data to all connected peers.  

```bash
/broadcast <data>
```

- **Deliver**  
Sends data directly to a specified peer.  

```bash
/deliver <peer_name> <data>
```

**Extension**  
Allows the use of protocol extensions. Multiple extensions are available:  

- **Extension 1**: Logs system statistics.  
  
  ```bash
  /extension 1 <payload>
  ```

- **Extension 2**: Validates and broadcasts a message.  
  
  ```bash
  /extension 2 "Valid: <payload>"
  ```

- **Extension 3**: Counts connected peers.  
  
  ```bash
  /extension 3 count
  ```

- **Extension 4**: Broadcasts system time.  
  
  ```bash
  /extension 4 time
  ```

- **Extension 5**: Filters and forwards messages containing 'important'.  
  
  ```bash
  /extension 5 <payload>
  ```

- **Extension 6**: Saves incoming messages containing 'Save'.  
  
  ```bash
  /extension 6 <payload>
  ```




## Tracker

You can start up a tracker by running `cargo run -p bing2bing-core --bin tracker`.
You can pass command line arguments in when using cargo: `cargo run -p bing2bing-core --bin tracker -- --host 0.0.0.0 --port 3001` (note the `--` after `--bin tracker`).

The tracker takes an ip address and port to listen on.
If you set the ip address to 0.0.0.0 it will listen on whatever ip addresses the machine responds to.


## Client

You can start up a simple client by running `cargo run -p bing2bing`.
The tui takes several command line arguments, and they are not entirely intuitive.

1. `--host` this is the ip address that "your" peer will listen on.
Note that you *cannot* set this to be 0.0.0.0 because this value is directly used when sending out protocol messages. However, this could be automated (e.g., as points that you could earn).

2. `--port` the port that your peer will listen on. Note that this must be unique for whatever machine you are running it on!

3. `--tracker-host` the ip address of a tracker to connect to and boostrap yourself into the network.

4. `--tracker-port` the port of the tracker to connect to.

5. `--name` the name that this peer will go by.

6. `--simple` if set, you will start up a simple client that only responds to `/say` and `/quit` input. If you don't set it, then a fancier UI will be used. For the fancy TUI, pressing `e` will put you into edit mode, pressing `esc` will bring you out of edit mode, typing `q` will quit, and pressing `l` or `h` will move between the two tabs.

