use std::fs::File;
use std::io::{self, Read, Write};
use serde_json::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Message {
    sender: String,
    message: String,
}

pub fn write_message(the_sender: &str,the_message: &str) -> io::Result<()> {

    //read the contents of the file into contents
    let mut file = File::open("chat.json")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    //deserializes contents into a serde_json Value
    let mut messages: Value = serde_json::from_str(&contents).unwrap_or(Value::Array(vec![]));

    //Create the new message
    let message = Message {
        sender: the_sender.to_owned(),
        message: the_message.to_owned(),
    };
    //pass the message into to_value function to get Value object
    let message_json = serde_json::to_value(message).unwrap();
    //push into messages
    messages.as_array_mut().unwrap().push(message_json);

    //serialize the updated messages to json
    let new_contents = serde_json::to_string_pretty(&messages)?;

    //write the updated json back to chat.json
    let mut file = File::create("chat.json")?;
    file.write_all(new_contents.as_bytes())?;

    Ok(())
}
