pub enum DecodeMode {
    Name,
    Message,
}

//function to extract the sender name and message from a hex output of a bundle
pub fn extract_source_message(the_string: String) -> Vec<String> {
    let mut output = Vec::new();
    let name_start = "201652f2f".to_string();
    let name_end = "2f8201652f".to_string();
    let text = get_text_between_strings(the_string.clone(), name_start, name_end,DecodeMode::Name);
    output.push(decode_hex(text));
    let message_start = "01010000".to_string();
    let message_end = "0aff".to_string();
    let text = get_text_between_strings(the_string, message_start, message_end,DecodeMode::Message);
    let mut decoded_hex = decode_hex(text);

    //when message > 31, 
    //2 more hexadecimal digits are allocated in between message and message start, 
    //which means another ascii letter is infront of the text
    //so remove(0) when length > 31
    if decoded_hex.len() > 31{
        decoded_hex.remove(0);
    }
    output.push(decoded_hex);
    output
}

//function to return a String between 2 Strings
fn get_text_between_strings(s: String, start: String, end: String, mode: DecodeMode) -> String {

    if let Some(start_idx) = s.find(&start) {
        if let Some(end_idx) = s[start_idx..].find(&end){ 
            match mode{
                DecodeMode::Name => {return s[start_idx + start.len()..start_idx + end_idx].to_string();}

                //remove first 2 hexadecimal digits
                DecodeMode::Message => {return s[start_idx + start.len() + 2..start_idx + end_idx].to_string();}
            }
        }
    }
    "Error: String Not found".to_string()
}

//function to decode a hex value into a ascii text
pub fn decode_hex(hex: String) -> String{
    let byte_vec = hex::decode(hex).unwrap();
    String::from_utf8(byte_vec).unwrap()
}

