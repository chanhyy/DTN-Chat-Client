use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::fs::File;
use std::io::{self, Read, Write};
use std::thread;
use std::sync::mpsc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::{error::Error};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, BorderType, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use core::result;
use unicode_width::UnicodeWidthStr;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
mod write;
mod decode;
mod processes;
enum ChatMode {
    Idle,
    Chatting,
}
enum State {
    Near,
    Away,
}
enum AppEvent<I> {
    Input(I),
    Tick,
}

impl ToString for State {
    fn to_string(&self) -> String {
        match self {
            State::Near => "Near".to_string(),
            State::Away => "Away".to_string(),
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
struct Message {
    sender: String,
    message: String,
}

struct SeenNode{
    name: String,
    state: State
} 
struct Application {
    input: String, 
    input_mode: ChatMode,
    messages: Vec<String>,
}

impl Default for Application {
    fn default() -> Application {
        Application {
            input: String::new(),
            input_mode: ChatMode::Idle,
            messages: Vec::new(),
        }
    }
}
impl SeenNode {
    fn new(name: String, state: State) -> Self {
        SeenNode { name, state }
    }
}

//global variable to keep track of the current selected node
static mut GLOBAL_SELECTED_NODE: String = String::new();
//static mut GLOBAL_LOCAL_USER: String = String::new();

fn main() -> result::Result<(), Box<dyn Error>> {
    //check if chat.json exists
    let chatfile = "chat.json";
    let chatfile_exists = std::path::Path::new(chatfile).exists();
    if !chatfile_exists {
        // create chat.json if doesnt exist
        let mut file = File::create(chatfile)?;
        //initializes chat.json with an empty array
        file.write_all(b"[]\n")?;
    }
    // Starts creating tui
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let app = Application::default();
    // calls run and loop until q is pressed
    let res = run(&mut terminal, app); 

    // to restore the terminal after quitting
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }
    print!("\x1B[2J\x1B[1;1H");
    println!("Terminal restored." );
    let dtnds = processes::dtndfind();
    for dtnd in dtnds{
        processes::dtndkill(dtnd);
        thread::sleep(Duration::from_secs(1));
    }
    println!("\u{2714}  Daemon shutdown successful. Thank you for using DTN Chat Client." );
    Ok(())
}

fn run<B: Backend>(terminal: &mut Terminal<B>, mut app: Application) -> io::Result<()> {
    let mut peer_list_state = ListState::default(); // create a ListState to track selected node
    peer_list_state.select(Some(0)); //select Some() initial value for ListState
    let mut peernames: Vec<_> = Vec::new() ;
    let mut seen_nodes: Vec<SeenNode> = Vec::new() ; // create Vec<SeenNode>
    //create HashMap(to memorize the index corresponds to the name of a peer in the peernames)
    let mut node_map= HashMap::new(); 
    let mut map_num: i32 = 0; // initialize value of index for node_map
    let mut notifications: Vec<String> = Vec::new();
    let tick = Duration::from_millis(250); // set up a tick rate for the "input reading thread"

    // create channel of communication between the thread and main thread, passes read values 
    let (tx, rx) = mpsc::channel(); 

    //spawn the thread that loops the reading task,executes the closure ||
    thread::spawn(move || {
        //initialize mutable last_tick
        let mut last_tick = Instant::now();
        loop {
            //calculates expire value
            let expire = tick
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            //wait event to occur,waot till expire
            if event::poll(expire).expect("Error Polling") {
                if let Event::Key(key) = event::read().expect("Error reading events") {
                    //tell main thread the read Key
                    tx.send(AppEvent::Input(key)).expect("Error sending Event");
                }
            }
            //send AppEvent::Tick if tick expires
            if last_tick.elapsed() >= tick && tx.send(AppEvent::Tick).is_ok() {
                last_tick = Instant::now();
            }
        }
    });
    loop {
        //draw the Frame<> in the closure with the value passed by calling ui()
        terminal.draw(|f| ui(f, &app, &mut peer_list_state, 
            &mut peernames,&mut seen_nodes,&mut node_map,&mut map_num, &mut notifications) )?;

        // match the Key read from thread with a corresponding action
        if let AppEvent::Input(key) = rx.recv().unwrap() {
            match app.input_mode {
                //different actions in different ChatModes
                ChatMode::Idle => match key.code {
                    KeyCode::Enter => {
                        app.input_mode = ChatMode::Chatting;
                    }
                    //point peer_list_state at the correct peer when up or down is pressed
                    KeyCode::Down => {
                        if let Some(selected) = peer_list_state.selected() {
                            if !peernames.is_empty(){
                                let peers_num = peernames.len();
                                if selected >= peers_num - 1 {
                                    peer_list_state.select(Some(0));
                                } else {
                                    peer_list_state.select(Some(selected + 1));
                                }  
                            }
                            
                        }
                    }
                    KeyCode::Up => {
                        if let Some(selected) = peer_list_state.selected() {
                            if !peernames.is_empty(){
                                let peers_num = peernames.len();
                                if selected == 0 {
                                    peer_list_state.select(Some(peers_num - 1));
                                } else {
                                    peer_list_state.select(Some(selected - 1));
                                }
                            }
                        }
                    }
                    //q to quit, goes back to main() to restore terminal.
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    //do nothing
                    _ => {}
                },
                ChatMode::Chatting => match key.code {
                    KeyCode::Enter => {
                        //get all String in the Send a message box and pass it to message
                        let message:String = app.input.drain(..).collect();
                        //avoid empty messages
                        if !message.is_empty(){
                            //add the messages Vec<>
                            app.messages.push(message.clone());
                            unsafe{
                                processes::send(GLOBAL_SELECTED_NODE.clone(), message.clone());
                                //create a special name of sender when sender is myself, to update my chat.json
                                let self_concat = GLOBAL_SELECTED_NODE.clone() + "(self)";
                                //write message
                                let _result = write::write_message(&self_concat,&message);
                            } 
                        }
                        
                    }
                    KeyCode::Backspace => {
                        //backspace effect
                        app.input.pop();
                    }
                    KeyCode::Esc => {
                        //quit ChatMode
                        app.input_mode = ChatMode::Idle;
                    }
                    KeyCode::Char(c) => {
                        //get character typed
                        app.input.push(c);
                    }
                    //do nothing
                    _ => {}
                },
            }
        }
    }
}
//passes bunch of initialized values from runapp()
fn ui<B: Backend>(f: &mut Frame<B>, 
                app: &Application, 
                peer_list_state: &mut ListState, 
                peernames: &mut Vec<ListItem>,
                seen_nodes: &mut Vec<SeenNode>,
                node_map: &mut HashMap<i32, String>,
                map_num: &mut i32,
                notifications: &mut Vec<String>){
    //calls peers() to get output from dtnquery to check neighbours
    let json_string = processes::peers();
    //skips "Listing of peers"
    let json_string = &json_string.lines().skip(1).collect::<Vec<&str>>().join("\n");
    //maps json values deserialized by serde_json
    let nodes: Map<String, Value> = serde_json::from_str(json_string).unwrap();
    //for every mapped value, push the name if its new
    for (name, _nodeinfo) in nodes.iter() {
        let new_name = ListItem::new(name.clone().to_string());
            if !peernames.contains(&new_name) &&
            !peernames.contains(&ListItem::new("\u{1F4E8} ".to_string() + &name.clone().to_string())){
                peernames.push(new_name.clone());
                //push to seen_nodes too to keep track of states
                let new_seen_node = SeenNode::new
                        (name.to_string(), State::Near);
                seen_nodes.push(new_seen_node);
            
                //update node_map and increment map_num
                node_map.insert(map_num.to_owned(),name.clone().to_string());
                *map_num += 1;
                
            }
        }
    //add notification icon if notification list contain the name.
    for (_, name) in node_map.iter() {
        if notifications.contains(&name.clone()) && 
            unsafe{!GLOBAL_SELECTED_NODE.eq(&name.clone())} &&
            !peernames.contains(&ListItem::new("\u{1F4E8} ".to_string() + &name.clone())){
            if let Some(index) = peernames.iter().position(|x| x == &ListItem::new(name.clone())) {
                let namewithnoti = "\u{1F4E8} ".to_string() + &name.clone();
                peernames[index] = ListItem::new(namewithnoti);
            }
            //remove notification icon when selected a name with notification
        }else if unsafe{GLOBAL_SELECTED_NODE.eq(&name.clone())}{
            if let Some(index) = peernames.iter()
                .position(|x| x == &ListItem::new("\u{1F4E8} ".to_string() + &name.clone())) {
                peernames[index] = ListItem::new(name.clone());
                notifications.retain(|r| r != &name.clone());
            }
        }
    }
    //update states list by matching with node_map, Near if matched, Away if otherwise
    let mut states: Vec<_> = Vec::new() ;
    if !seen_nodes.is_empty(){
        for seen_node in seen_nodes{
            let mut found = false;
            for (name, _nodeinfo) in nodes.iter() {
                if seen_node.name == *name{
                    found = true
                }
            }
            if !found{
                seen_node.state = State::Away;
            } else{
                seen_node.state = State::Near;
            }
            let state : String = seen_node.state.to_string();
            let new_state = ListItem::new(state)
                .style(match seen_node.state {
                    State::Near => Style::default().fg(Color::Green),
                    State::Away => Style::default().fg(Color::Red),
                });
            states.push(new_state);
            
        }

    }
    
    //update GLOBAL_SELECTED_NODE    
    if !node_map.is_empty(){
        let to_find = peer_list_state.selected().unwrap() as i32;
        if let Some(string) = node_map.get(&to_find) {
            unsafe {
                GLOBAL_SELECTED_NODE = string.to_owned(); 
            }
        } else {
            println!("Error finding the key.");
        }
        
    }

    //check for new message using receive()
    let new_message = processes::receive();
    //if theres a message
    if !new_message.is_empty(){
        //get the sender's name and the message
        let name_and_message = decode::extract_source_message(new_message);
        //updates chat log chat.json
        let _result = write::write_message(&name_and_message[0],&name_and_message[1]);
        let name = &name_and_message[0];
        if !notifications.contains(name) && unsafe{!GLOBAL_SELECTED_NODE.eq(name)}{
            notifications.push(name.to_string())
        }
    }
    
    //Gathering chat log data
    // Read the contents of the file into a string: contents to display later in ListItems
    let mut file = match File::open("chat.json") {
        Ok(file) => file,
        Err(e) => panic!("Unable to open file: {:?}", e),
    };
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    //deserialize the json string into a Vec<Message> 
    let messages: Vec<Message> = serde_json::from_str(&contents).unwrap_or(Vec::new());

    //create the ListItems to be displayed, iterates every message in chat.json, 
    //filters only the messages from selected node and my mesages to the selected node
    let message_list: Vec<ListItem> = messages.iter()
        .filter(|message| {
            unsafe{
                let sender = &message.sender;
                sender == &GLOBAL_SELECTED_NODE.clone() || sender == &(GLOBAL_SELECTED_NODE.clone() + "(self)") 
            }   
        })
        .map(|message| {
            let sender = &message.sender;
            unsafe{
                if sender == &(GLOBAL_SELECTED_NODE.clone() + "(self)"){
                    let msg = vec![
                    Span::styled("You", Style::default()
                        .add_modifier(Modifier::BOLD)),
                    Span::raw(" : "),
                    Span::raw( &message.message)];
                    let text = Text::from(Spans::from(msg));
                    ListItem::new(text)
                } else {
                    let msg = vec![
                    Span::styled(&message.sender, Style::default()
                        .add_modifier(Modifier::BOLD)),
                    Span::raw(" : "),
                    Span::raw( &message.message)];
                    let text = Text::from(Spans::from(msg));
                    ListItem::new(text)
                } 
            }
        })
        .collect(); 
    //==========everything ready to be put in widgets at this point==========

    let biggerchunks = Layout::default()
    .margin(1)
    .direction(Direction::Vertical)
    .constraints(
        [
            Constraint::Length(1),
            Constraint::Min(10)
        ]
        .as_ref(),
    )
    .split(f.size());

    //define layouts
    let bigchunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Length(10),
                Constraint::Min(20)
            ]
            .as_ref(),
        )
        .split(biggerchunks[1]);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(3)
            ]
            .as_ref(),
        )
        .split(bigchunks[2]);

    
    // Surrounding block
    let bigblock = Block::default()
        .border_style(Style::default().fg(Color::LightGreen))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(" DTN Chat Client ")
        .title_alignment(Alignment::Center);
    f.render_widget(bigblock, f.size());
    
    //display list of seen peers
    let list = List::new(peernames.to_owned())
            .block(Block::default()
            .borders(Borders::ALL)
            .title("Seen Peers")
            .title_alignment(Alignment::Center)
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD),);
    f.render_stateful_widget(list, bigchunks[0],peer_list_state);

    //display list of States
    let statelist = List::new(states.to_owned())
            .block(Block::default()
            .borders(Borders::ALL)
            .title("State")
            .title_alignment(Alignment::Center)
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD),);
    f.render_widget(statelist, bigchunks[1]);
    
    //the message in top right
    //get Vec<Span> and style to be rendered
    let (msg, style) = match app.input_mode {
        ChatMode::Idle => 
            if peernames.is_empty(){
                (vec![
                Span::raw("Press "),
                Span::styled("q", Style::default()
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::ITALIC)),
                Span::raw(" to exit or wait for a peer to chat with")],
                Style::default())
            } else{
            (vec![
                Span::raw("Press "),
                Span::styled("q", Style::default()
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::ITALIC)),
                Span::raw(" to exit, "),
                Span::styled("Enter", Style::default()
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::ITALIC)),
                Span::raw(" to send a message"),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK))
            }
        ,
        ChatMode::Chatting => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default()
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::ITALIC)),
                Span::raw(" to go back, "),
                Span::styled("Enter", Style::default()
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::ITALIC)),
                Span::raw(" to send the message"),
            ],
            Style::default(),
        ),
    };

    

    //create the message and patch the style 
    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);
    
    let instructions = Paragraph::new("\u{1F53A}\u{1F53B} Arrow keys to select peers").alignment(Alignment::Right);
    f.render_widget(instructions, chunks[0]);
    let prompt = Paragraph::new(text).alignment(Alignment::Right);
    f.render_widget(prompt, chunks[1]);

    let current_user = Paragraph::new(format!("Current user: {}", processes::hostname()))
        .alignment(Alignment::Left).style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(current_user, biggerchunks[0]);

    
    let mut chat_list_state = ListState::default(); // create a ListState to track selected node
    
    if !message_list.is_empty(){
        let messages_num = message_list.len();
        chat_list_state.select(Some(messages_num - 1)); //select Some() initial value for ListState
    }
    
    
    //display the list of messages
    let messages = List::new(message_list)
        .block(Block::default()
        .borders(Borders::ALL)
        .title("Chat log")
        .title_alignment(Alignment::Center));
    f.render_stateful_widget(messages, chunks[2],&mut chat_list_state);

    //send a message box
    let input = Paragraph::new(app.input.as_ref())
        .style(match app.input_mode {
            ChatMode::Idle => Style::default(),
            ChatMode::Chatting => Style::default().fg(Color::Yellow),
        })
        .block(Block::default()
        .borders(Borders::ALL)
        .title("Send a message")
        .title_alignment(Alignment::Center));
    f.render_widget(input, chunks[3]);

    match app.input_mode {
        ChatMode::Idle =>
            {} // to hide the keyboard cursor
        ChatMode::Chatting => {
            //set cursor at the right place when chatting
            f.set_cursor(
                chunks[3].x + app.input.width() as u16 + 1,
                chunks[3].y + 1,
            )
        }
    }

}

