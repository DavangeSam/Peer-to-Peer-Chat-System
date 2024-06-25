//! This module provides a fancy text-based user interface (TUI) for the Bing2Bing application.
//! It allows users to interact with the Bing2Bing network, send messages, and view logs.

use bing2bing_core::Bing2BingFrame;
use bing2bing_core::{Bing2BingError, Client, ClientServerMessage, Server};
use chrono::Local;
use chrono::{TimeZone, Utc};
use std::{
    collections::VecDeque,
    io::Stdout,
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tracing::{debug, error, trace};
use unicode_width::UnicodeWidthStr;

use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Terminal,
};

use tui_logger::TuiLoggerWidget;

use crate::{Cli, UiClientMessage};

/// The type for the channel that receives messages from the UI client.
pub type UiClientRxChannel = mpsc::UnboundedReceiver<UiClientMessage>;

/// Represents the input mode of the application.
#[derive(Debug, Clone, Copy)]
pub(crate) enum InputMode {
    /// Normal mode, where the user can navigate the menu.
    Normal,
    /// Editing mode, where the user can input text.
    Editing,
}

/// Represents the events that can occur in the application.
#[derive(Debug)]
enum Event<I> {
    /// An input event, such as a key press.
    Input(I),
    /// A tick event, indicating a periodic update.
    Tick,
}

/// Starts the fancy TUI application.
///
/// # Arguments
///
/// * `args` - The command-line arguments for configuring the application.
///
/// # Returns
///
/// Returns `Ok(())` if the application starts successfully, or an error if it fails.
pub async fn start(args: Cli) -> Result<(), Bing2BingError> {
    // //  tracing_subscriber::registry()
    // //   .with(tui_logger::tracing_subscriber_layer())
    // //   .init();
    tui_logger::init_logger(log::LevelFilter::Trace).unwrap();
    tui_logger::set_default_level(log::LevelFilter::Info);

    // // this is really bad, but I can't figure out a better way to do it because the Tui widget
    // // uses the `Log` crate and tokio uses `tracing`. There *are* a bunch of compatibility
    // // layers and apis that are available for `tracing` but I have not figured out how
    // // to have things as nice as they are in the simple ui being set via environment
    // // variables.
    // //
    // // Regardless, it is more interseting to have the log level filtering happening
    // // in the UI, see [tui_logger](https://docs.rs/tui-logger/0.6.3/tui_logger/index.html)
    // // for a starting poitn to do cool stuff.

    tui_logger::set_level_for_target("bing2bing_core", log::LevelFilter::Trace);
    tui_logger::set_level_for_target("bing2bing_core::server", log::LevelFilter::Trace);
    tui_logger::set_level_for_target("bing2bing_core::client", log::LevelFilter::Trace);
    tui_logger::set_level_for_target("bing2bing_core::cmd", log::LevelFilter::Trace);
    tui_logger::set_level_for_target("bing2bing_core::peer", log::LevelFilter::Trace);
    tui_logger::set_level_for_target("bing2bing_core::connection", log::LevelFilter::Trace);
    tui_logger::set_level_for_target("bing2bing_core::peer_map", log::LevelFilter::Trace);

    // let args = cli;

    trace!("Args: {:?}", args);

    let ip_address = args.ip_address.to_string().clone();
    let port = args.port; //.to_string().clone();

    let tracker_ip_address = args.tracker_ip_address.to_string().clone();
    let tracker_port = args.tracker_port; //.to_string().clone();

    let my_name = args.name;

    let max_connections = args.max_connections;

    let (ui_client_tx, ui_client_rx) = mpsc::unbounded_channel();

    let (client, server) = bing2bing_core::init(&my_name, &ip_address, port).await;

    let network_client = client;

    let app = App::new();

    let moved_app = app.clone();
    std::thread::spawn(move || {
        debug!("STARTING PEER!!!!");
        start_peer(
            moved_app,
            network_client,
            server,
            tracker_ip_address,
            tracker_port,
            max_connections,
            ui_client_rx,
        )
    });

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let tick_rate = Duration::from_millis(200);

    // this is a thread that will fire ticks
    // to let us know that we need to update things.
    // It essentiallyc controls the refresh rate of how often we will
    // scan for input from the user.
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll failed!") {
                if let CEvent::Key(key) = event::read().expect("can't read events?") {
                    // tx.send(Event::Input(key)).expect("can't send events?");
                    if let Err(err) = tx.send(Event::Input(key)) {
                        error!(
                            "Had an error when trying to send to input key handler; breaking. {}",
                            err
                        );
                        break;
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate && tx.send(Event::Tick).is_ok() {
                last_tick = Instant::now();
            }
        }
    });

    enable_raw_mode().expect("Can't start raw mode?!?!");

    let mut stdout = std::io::stdout();
    execute!(stdout, Clear(ClearType::All))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Arc::new(Mutex::new(Terminal::new(backend)?));
    let app_moved = app.clone();
    let terminal_moved = terminal.clone();

    tokio::spawn(async move {
        let app = app_moved;

        let terminal = terminal_moved;

        loop {
            match rx.recv().await {
                Some(Event::Input(event)) => {
                    match app.get_input_mode() {
                        InputMode::Editing => {
                            match event.code {
                                // KeyCode::Char('\n') => {
                                KeyCode::Enter => {
                                    debug!("got Enter");

                                    let to_say = app.input_string_drain();
                                    app.add_message(&to_say);

                                    let msg = if to_say.starts_with("/whisper ") {
                                        trace!("input line started with /whisper!");
                                        let to_say = to_say
                                            .clone()
                                            .strip_prefix("/whisper ")
                                            .unwrap()
                                            .to_string();
                                        let (to, message) = to_say.split_once(' ').unwrap();

                                        UiClientMessage::Whisper(
                                            to.to_string(),
                                            message.to_string(),
                                        )
                                    } else if to_say.starts_with("/ping ") {
                                        trace!("input line started with /ping !");
                                        let to = to_say
                                            .clone()
                                            .strip_prefix("/ping ")
                                            .unwrap()
                                            .to_string();

                                        let now: u64 =
                                            Utc::now().timestamp_millis().try_into().unwrap();
                                        UiClientMessage::Ping(to.to_string(), now)
                                    } else if to_say.starts_with("/extension ") {
                                        let to_say = to_say
                                            .clone()
                                            .strip_prefix("/extension ")
                                            .unwrap()
                                            .to_string();
                                        let (id, payload) = to_say.split_once(' ').unwrap();
                                        let id: u64 = id.parse().unwrap();
                                        UiClientMessage::Extension(id, payload.to_string())
                                    } else if to_say.starts_with("/deliver "){
                                        let to_say = to_say
                                            .clone()
                                            .strip_prefix("/deliver ")
                                            .unwrap()
                                            .to_string();
                                        let (to, data) = to_say.split_once(' ').unwrap();
                                        let data = data.as_bytes().to_vec();
                                        UiClientMessage::Deliver(to.to_string(), data)
                                    } else {
                                        debug!("calling client.say()");

                                        UiClientMessage::Say(to_say)
                                    };

                                    ui_client_tx.send(msg).unwrap();
                                }
                                KeyCode::Char(c) => {
                                    trace!("got a character: {}", c);
                                    app.input_string_push(c);
                                }
                                KeyCode::Backspace => {
                                    app.input_string_pop();
                                }
                                KeyCode::Esc => {
                                    app.set_input_mode(InputMode::Normal);
                                }
                                _ => {}
                            }
                        }
                        InputMode::Normal => match event.code {
                            KeyCode::Char('e') => {
                                app.set_input_mode(InputMode::Editing);
                            }

                            KeyCode::Char('q') => {
                                debug!("we are shuttong down?!?! {}", app.is_shutdown());
                                disable_raw_mode().unwrap();
                                {
                                    let mut terminal = terminal.lock().unwrap();
                                    terminal.show_cursor().unwrap();
                                }

                                app.shutdown();
                                debug!("After calling shutdown() {}", app.is_shutdown());

                                break;
                            }
                            KeyCode::Char('h') => app.set_active_menu_item(MenuItem::Home),
                            KeyCode::Char('l') => app.set_active_menu_item(MenuItem::Logs),
                            _ => {}
                        },
                    }
                }

                Some(Event::Tick) => { /* we just do nothing here */ }
                None => {
                    break;
                }
            }
        }
    });

    loop {
        if app.is_shutdown() {
            debug!("App was shut down");
            break;
        }

        draw(&mut app.clone(), &mut terminal);
    }

    Ok(())
}

/// Draws the user interface on the terminal.
///
/// # Arguments
///
/// * `app` - The application state.
/// * `terminal` - The terminal backend.
pub fn draw(app: &mut App, terminal: &mut Arc<Mutex<Terminal<CrosstermBackend<Stdout>>>>) {
    //todo!("need to fix!");

    let menu_titles = vec!["Home", "Logs"];
    let mut terminal = terminal.lock().unwrap();
    terminal
        .draw(|rect| {
            let size = rect.size();

            let chunks = build_chunks(size);

            let input_mode = app.get_input_mode();

            let (msg, style) = get_input_mode_message(input_mode);

            let text = Line::from(msg);

            let _ = text.patch_style(style);

            let input_mode = app.get_input_mode();

            let input = get_input_box(app.clone_input_string(), input_mode);

            rect.render_widget(input, chunks[2]);

            let input_mode = app.get_input_mode();

            match input_mode {
                InputMode::Normal =>
                    // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
                    {}

                InputMode::Editing => {
                    // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
                    rect.set_cursor(
                        // Put cursor past the end of the input text
                        chunks[2].x + app.input_string_width() as u16 + 1,
                        // Move one line down, from the border to the input line
                        chunks[2].y + 1,
                    )
                }
            }

            // HACK
            // we really need to use a stateful widget and scroll here, but for the moment,
            // we will just truncate from the front of the messages when there are more
            // of them than can fit on screen (this will work weird with wrap btw)
            let num_ui_lines = (chunks[1].bottom() - chunks[1].top() - 2) as usize;
            if num_ui_lines < app.num_messages() {
                let num_messages_to_drop = app.num_messages() - num_ui_lines;

                app.drain_messages(num_messages_to_drop);
            }

            let message_list = app
                .message_list()
                .iter()
                .cloned()
                .map(|s| {
                    let content = vec![Line::from(Span::raw(s))];
                    ListItem::new(content)
                })
                .collect::<Vec<_>>();

            let message_list = List::new(message_list)
                .block(Block::default().borders(Borders::ALL).title("Messages"));
            //         let menu = menu_titles
            //             .iter()
            //             .map(|t| {
            //                 let (first, rest) = t.split_at(1);
            //                 Spans::from(vec![
            //                     Span::styled(
            //                         first,
            //                         Style::default()
            //                             .fg(Color::Yellow)
            //                             .add_modifier(Modifier::UNDERLINED),
            //                     ),
            //                     Span::styled(rest, Style::default().fg(Color::White)),
            //                 ])
            //             })
            //             .collect();

            let tabs = Tabs::new(menu_titles)
                .select(app.get_active_menu_item().into())
                .block(Block::default().title("Menu").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow))
                .divider(Span::raw("|"));

            match app.get_active_menu_item() {
                MenuItem::Home => {
                    rect.render_widget(message_list, chunks[1]);
                }
                MenuItem::Logs => {
                    let tui_w: TuiLoggerWidget = TuiLoggerWidget::default()
                        .block(
                            Block::default()
                                .title("Independent Tui Logger View")
                                .border_style(Style::default().fg(Color::White).bg(Color::Black))
                                .borders(Borders::ALL),
                        )
                        .style(Style::default().fg(Color::White).bg(Color::Black));
                    rect.render_widget(tui_w, chunks[1]);
                }
            }

            rect.render_widget(tabs, chunks[0]);
            
            rect.render_widget(Paragraph::new("BING2BING"), chunks[3]);
        })
        .unwrap();
}

/// Creates an input box widget with the given input string and input mode.
///
/// # Arguments
///
/// * `input_string` - The current input string.
/// * `input_mode` - The current input mode.
///
/// # Returns
///
/// Returns a `Paragraph` widget representing the input box.
fn get_input_box(input_string: String, input_mode: InputMode) -> Paragraph<'static> {
    Paragraph::new(input_string)
        .style(match input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL).title("Input"))
}

/// Returns the appropriate message and style for the given input mode.
///
/// # Arguments
///
/// * `input_mode` - The current input mode.
///
/// # Returns
///
/// Returns a tuple containing the message as a vector of `Span`s and the corresponding `Style`.
fn get_input_mode_message(input_mode: InputMode) -> (Vec<Span<'static>>, Style) {
    match input_mode {
        InputMode::Normal => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to start editing."),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::Editing => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to stop editing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to record the message"),
            ],
            Style::default(),
        ),
    }
}

/// Builds the layout chunks for the application.
///
/// # Arguments
///
/// * `size` - The size of the terminal.
///
/// # Returns
///
/// Returns a reference-counted slice of `Rect` representing the layout chunks.
fn build_chunks(size: Rect) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(3),
                Constraint::Length(5),
            ]
            .as_ref(),
        )
        .split(size)
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum MenuItem {
    Home,
    Logs,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Logs => 1,
        }
    }
}

#[tokio::main]
async fn start_peer(
    app: App,
    client: Client,
    server: Server,
    tracker_ip_address: String,
    tracker_port: u16,
    max_incoming_connections: u64,
    mut ui_rx: UiClientRxChannel,
) {
    trace!("Starting peer...");
    tokio::spawn(async move {
        server
            .start(
                &tracker_ip_address,
                &tracker_port.to_string(),
                max_incoming_connections,
            )
            .await
            .unwrap_or_else(|e| {
                debug!("Server shut down: {}", e);
            });
    });

    let moved_client = client.clone();
    tokio::spawn(async move {
        loop {
            if let Some(message_from_ui) = ui_rx.recv().await {
                trace!("Received {:?} from Ui", message_from_ui);
                match message_from_ui {
                    UiClientMessage::Ping(to, sent_at) => {
                        moved_client.ping(&to, sent_at).await;
                    }
                    UiClientMessage::Whisper(to, message) => {
                        moved_client.whisper(&to, &message).await;
                    }
                    UiClientMessage::Say(message) => {
                        moved_client.say(&message).await;
                    }
                    UiClientMessage::Extension(id, payload) => {
                        let payload_frame = Bing2BingFrame::Text(payload);
                        moved_client.send_extension(id, payload_frame).await
                    }
                    UiClientMessage::Deliver(to, data) => {
                        moved_client.deliver(&to, data).await;
                    }
                }
            }
        }
    });

    let x = tokio::spawn(async move {
        loop {
            trace!("Waiting for next message from client");
            let from_server_message = client.next_message().await;

            match from_server_message {
                ClientServerMessage::Ping(_) => {
                    panic!("Received a ping message from the server! Shouldn't happen!");
                }
                ClientServerMessage::Pong((from, sent_at)) => {
                    let now = Utc::now();

                    let millis: i64 = sent_at.try_into().unwrap();
                    let then = chrono::Utc.timestamp_millis_opt(millis).single().unwrap();

                    let latency = now.time() - then.time();

                    let formatted_pong = format!(
                        "[{}] PONG response from {}; latency: {}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
                        latency,
                    );
                    app.add_message(&formatted_pong);
                }
                ClientServerMessage::Whisper((from, _to, message)) => {
                    let formatted_whisper = format!(
                        "[{}] {} whispered to you: {}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
                        message
                    );
                    app.add_message(&formatted_whisper);
                }
                ClientServerMessage::Say((from, message)) => {
                    let formatted_say = format!(
                        "[{}] {}: {}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
                        message
                    );
                    app.add_message(&formatted_say);
                }
                ClientServerMessage::Extension((_source, id, payload)) => {
                    let formatted_extension = format!(
                        "[{}] sent extension {}: {:?}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        id,
                        payload
                    );
                    app.add_message(&formatted_extension);
                }
                ClientServerMessage::Deliver((from, to, data)) => {
                    let formatted_deliver = format!(
                        "[{}] {} delivered to {}: {:?}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
                        to,
                        data
                    );
                    app.add_message(&formatted_deliver);
                }
                ClientServerMessage::Broadcast((from, data)) => {
                    let formatted_broadcast = format!(
                        "[{}] {} broadcasted: {:?}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
                        data
                    );
                    app.add_message(&formatted_broadcast);
                }
            }
        }
    });

    x.await.unwrap();
}

#[derive(Debug, Clone)]
pub struct App {
    shared: Arc<Shared>,
}

impl App {
    pub fn new() -> Self {
        let shared = Shared::new();
        Self {
            shared: Arc::new(shared),
        }
    }

    pub(crate) fn get_input_mode(&self) -> InputMode {
        self.shared.state.lock().unwrap().input_mode
    }

    pub fn input_string_push(&self, c: char) {
        self.shared.state.lock().unwrap().input_string.push(c);
    }

    pub fn input_string_pop(&self) -> Option<char> {
        self.shared.state.lock().unwrap().input_string.pop()
    }

    pub fn input_string_drain(&self) -> String {
        self.shared
            .state
            .lock()
            .unwrap()
            .input_string
            .drain(..)
            .collect::<String>()
    }

    /// Add a message that will be displayed in the main ui frame.
    pub fn add_message(&self, message: &str) {
        self.shared
            .state
            .lock()
            .unwrap()
            .messages
            .push_back(message.to_string())
    }

    /// Get the number of messages we have in our buffer
    pub fn num_messages(&self) -> usize {
        self.shared.state.lock().unwrap().messages.len()
    }

    /// Switch between input modes (edit and regular)
    pub(crate) fn set_input_mode(&self, input_mode: InputMode) {
        self.shared.state.lock().unwrap().input_mode = input_mode;
    }

    pub(crate) fn get_active_menu_item(&self) -> MenuItem {
        self.shared.state.lock().unwrap().active_menu_item
    }

    pub(crate) fn set_active_menu_item(&self, menu_item: MenuItem) {
        self.shared.state.lock().unwrap().active_menu_item = menu_item
    }

    pub fn drain_messages(&self, n: usize) {
        drop(self.shared.state.lock().unwrap().messages.drain(..n))
    }

    pub fn input_string_width(&self) -> usize {
        self.shared.state.lock().unwrap().input_string.width()
    }

    pub fn clone_input_string(&self) -> String {
        self.shared.state.lock().unwrap().input_string.clone()
    }

    pub fn is_shutdown(&self) -> bool {
        self.shared.state.lock().unwrap().shut_down
    }

    pub fn shutdown(&self) {
        self.shared.state.lock().unwrap().shut_down = true;
    }

    pub fn message_list(&self) -> Vec<String> {
        let messages = &self.shared.state.lock().unwrap().messages;

        messages
            .iter()
            .enumerate()
            .map(|(i, v)| format!("{}: {}", i, v))
            .collect()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the shared state of the application.
#[derive(Debug)]
struct Shared {
    /// The mutex-protected application state.
    state: Mutex<State>,
}

impl Shared {
    /// Creates a new instance of the shared state.
    pub fn new() -> Self {
        let state = State::new();

        Self {
            state: Mutex::new(state),
        }
    }
}

/// Represents the internal state of the application.
#[derive(Debug)]
struct State {
    /// The list of messages.
    messages: VecDeque<String>,
    /// The current input mode.
    input_mode: InputMode,
    /// The currently active menu item.
    active_menu_item: MenuItem,
    /// The current input string.
    input_string: String,
    /// A flag indicating whether the application should shut down.
    shut_down: bool,
}

impl State {
    /// Creates a new instance of the application state.
    fn new() -> Self {
        State {
            messages: VecDeque::new(),
            input_mode: InputMode::Normal,
            active_menu_item: MenuItem::Home,
            input_string: String::new(),
            shut_down: false,
        }
    }
}
