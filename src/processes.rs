use std::{process::{Command, Stdio}};

pub fn hostname() -> String{
    //same logic as send()
    //tells the dtn daemon to receive message
    let name = Command::new("hostname")
        .output()
        .expect("failed to execute process");

    let stdout = String::from_utf8_lossy(&name.stdout);
    let _stderr = String::from_utf8_lossy(&name.stderr);
    
    stdout.into_owned()
}
pub fn dtndkill(id: String){
    let _kill = Command::new("kill")
    .arg(&id)
    .spawn();
    println!("Killing PID: {}", id);
}   

pub fn dtndfind() -> Vec<String>{
    let mut dtnds:Vec<String> = Vec::new();
    let output = Command::new("sh")
                      .arg("-c")
                      .arg("ps -A | grep dtnd")
                      .output()
                      .expect("failed to execute ps command");

    // Convert the output into a string slice
    let output_str = std::str::from_utf8(&output.stdout).unwrap();

    // Split the string into lines and process each line
    for ps in output_str.lines() {
        // Split each line into fields and obtain the PID field
        let fields: Vec<&str> = ps.split_whitespace().collect();
        let pid = fields[0];
        dtnds.push(pid.to_string());
        // Print the PID
        println!("dtnd process found with PID: {}", pid);
    }   
    dtnds
}
pub fn send(name: String,  message: String)  -> String{
    //create new process that runs echo and puts result in echo_output
    let echo_output = Command::new("echo")
        .arg(message)
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute process");

    //creates target String dtn://(nodename)/incoming
    let mut target = String::from("dtn://");
        target.push_str(name.as_str().trim()); 
        target.push_str("/incoming");

    //creates new process that tells the dtn daemon to send message
    let dtnsend_output = Command::new("dtnsend")
        .arg("-r")
        .arg(target)
        .stdin(echo_output.stdout.unwrap())
        .output()
        .expect("failed to execute process");

    if dtnsend_output.status.success() {
        let stdout = String::from_utf8_lossy(&dtnsend_output.stdout);
        stdout.into_owned()
    } else {
        let stderr = String::from_utf8_lossy(&dtnsend_output.stderr);
        stderr.into_owned()
    }
 
}

pub fn receive() -> String{
    //same logic as send()
    //tells the dtn daemon to receive message
    let dtnrecv_output = Command::new("dtnrecv")
        .arg("-e")
        .arg("incoming")
        .arg("-x")
        .output()
        .expect("failed to execute process");

    let stdout = String::from_utf8_lossy(&dtnrecv_output.stdout);
    let _stderr = String::from_utf8_lossy(&dtnrecv_output.stderr);
    
    stdout.into_owned()
}
pub fn peers() -> String{

    //same logic as send()
    //tells the dtn daemon to get peers list
    let output = Command::new("dtnquery")
        .arg("peers")
        .output()
        .expect("failed to execute process");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.into_owned()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        stderr.into_owned()
    }
}


// pub fn dtndstart(username: String){
//     let command = format!("dtnd -n {} -e incoming -C mtcp -r epidemic -p 3s",username.trim());
//     let _startup = Command::new("gnome-terminal")
//                       .arg("--")
//                       .arg("sh")
//                       .arg("-c")
//                       .arg(format!("{}", command))
//                       .output()
//                       .expect("failed to execute dtnd");
    
// }