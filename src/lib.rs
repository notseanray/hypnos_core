use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::Command,
};

use serenity::{model::id::ChannelId, prelude::*};

// not so useful help message to printout when accessed from the command line
pub fn print_help() {
    let help_msg = format!(
        "
       hypnos_core: chat bot, backup manager, and more
       Usage:
            hypnos_core
            hypnos_core <action> [option]
            hypnos_core <stop [all/<server>] | start [all/<server>]>
       Options:
            gen_cfg
            health
            backup [ls | rm] <backup>
       "
    );
    println!("{}", help_msg);
    std::process::exit(0);
}

// this may be very messy, but it's the easiest way to do this (iterating with a loop and substring
// could work, but I'm lazy)
//
// plus I trust llvm to make my code less bad
//
// This removes all the formmating codes coming from MC chat
fn replace_formatting(msg: String) -> String {
    msg.replace(&"§0", "")
        .replace(&"§1", "")
        .replace(&"§2", "")
        .replace(&"§3", "")
        .replace(&"§4", "")
        .replace(&"§5", "")
        .replace(&"§6", "")
        .replace(&"§7", "")
        .replace(&"§8", "")
        .replace(&"§9", "")
        .replace(&"§a", "")
        .replace(&"§b", "")
        .replace(&"§c", "")
        .replace(&"§d", "")
        .replace(&"§e", "")
        .replace(&"§f", "")
        .replace(&"§g", "")
        .replace(&"§k", "")
        .replace(&"§l", "")
        .replace(&"§m", "")
        .replace(&"§n", "")
        .replace(&"§o", "")
        .replace(&"§r", "")
}

// generate the tmux pipe connecting to the specified server, this also takes in the option to
// delete the file if it exists before generating it
// that can be used at startup or when just resetting the file in general
pub async fn gen_pipe(server_name: String, rm: bool) {
    if rm {
        // get the file path to the pipe file
        let pipe_old: String = format!("/tmp/{}-HypnosCore", &server_name);

        // remove the old pipe file if it exists
        if check_dir(pipe_old.to_owned()) {
            Command::new("rm")
                .arg(&pipe_old)
                .spawn()
                .expect("*error: failed to delete pipe file");
        }
    }

    // create the tmux command that will be entered to set the pipe
    let pipe_output = format!("cat >/tmp/{}-HypnosCore", &server_name);
    Command::new("tmux")
        .args(["pipe-pane", "-t", &server_name, &pipe_output])
        .spawn()
        .expect("*error: failed to generate pipe file");
}

// small function to send a command to the specific tmux session, this replaces new lines due to it
// causing a problem with commands
//
// this is one of the limitations of this system, but it's not that bad because if there are
// multiple lines you can send the command multiple times
pub async fn send_command(server_name: String, message: String) {
    Command::new("tmux")
        .args([
            "send-keys",
            "-t",
            &server_name,
            &message.replace("\n", ""),
            "Enter",
        ])
        .spawn()
        .expect("*error: failed to send to tmux session");
}

// function to check if the file or folder exist, emits a warning if non absolute paths are used
// absolute paths are always prefered since they're just easier to work with and makes thing more
// portable/less reliant on each other
pub fn check_dir(dir: String) -> bool {
    let current_path = PathBuf::from(&dir);
    if !current_path.is_absolute() {
        println!("*warn: non absolute path used at: {}", dir);
    }
    if current_path.exists() {
        return true;
    }
    false
}

// checks the number of lines in the log file to set them initially, this prevents old messages
// from being spat out if the bot restarts (and makes it a lot less annoying)
pub fn set_lines(server_name: String) -> usize {
    let file_path: String = format!("/tmp/{}-HypnosCore", &server_name);
    let file = File::open(&file_path).unwrap();
    let reader = BufReader::new(file);

    // count the amount of lines in the log file
    reader.lines().count()
}

// function to handle mc commands send to a tmux session, this has a command "whitelist" to ensure
// that only certain commands are executed
async fn handle_command(server_name: String, cmd: String, arg: String) {
    let mut final_cmd = String::new();
    match &cmd[..] {
        "s" => final_cmd = format!("scoreboard objectives setdisplay sidebar {}", arg),
        "score" => final_cmd = format!("scoreboard objectives setdisplay sidebar {}", arg),
        _ => final_cmd = "list".to_string(),
    }
    send_command(server_name.to_owned(), final_cmd).await;
}

// update messages from the log file, this takes in the log file, checks if the lines can be
// ignored, then checks if the new lines are in game commands, if they are then use handle command
// to check them and if not send them to discord
//
// unfortunately this is not very efficient but honestly I don't really care, this runs on separate
// threads from the mc server and if the log file gets above 2k lines it gets repiped with tmux to
// prevent the function from taing too long
pub async fn update_messages(
    ign_prefix: String,
    server_name: String,
    lines: usize,
    ctx: Context,
    chat_id: u64,
) -> usize {
    let file_path: String = format!("/tmp/{}-HypnosCore", &server_name);

    // open the log file in bufreader
    let file = File::open(&file_path).unwrap();
    let reader = BufReader::new(file);

    let mut cur_line: usize = lines;

    // Read the file line by line using the lines() iterator from std::io::BufRead.
    for (i, line) in reader.lines().enumerate() {
        // skip lines that are irrelevant
        if i > cur_line {
            // if they are new, update the counter
            cur_line = i;

            let line = line.unwrap();

            // if the line is too short then skip it
            if &line.chars().count() < &35 {
                continue;
            }

            // check if the message starts with certain characters
            let line_sep: &str = &line[33..];
            if !line.starts_with("[") || (!line_sep.starts_with("<") && !line_sep.starts_with("§"))
            {
                continue;
            }

            // check if it's an in game command
            let ign_command: String = format!("§r> {}", ign_prefix);

            // if it is, then check if it's in the command "whitelist"
            if line_sep.contains(&ign_command) {
                let allowed_commands: Vec<String> = vec!["s".to_string(), "score".to_string()];

                // parse where the actual command starts, without the username
                let cmd_start: usize = line_sep.find(&ign_command).unwrap() + 5;

                // parse the actual command with argument
                let cmd: &str = &line_sep[cmd_start..];

                // parse where the just the command ends and where the argument is
                let cmd_split: usize = cmd.find(" ").unwrap();

                // if it's in the whitelist send the command and argument separately to handle
                // command, there it will be transformed to the correct in game equivalent
                if allowed_commands.contains(&cmd[1..cmd_split].to_owned()) {
                    handle_command(
                        server_name.to_owned(),
                        cmd[1..cmd_split].to_owned(),
                        cmd[(cmd_split + 1)..].to_owned(),
                    )
                    .await;
                }

                // update the line count in the main file and continue
                return cur_line;
            }

            // if it's not an in game command, we can generate what the discord message will be
            //
            // firstly we put the server name then the new line message, this is where replace
            // formatting comes in to remove the special mc escape sequences
            let message = format!(
                "`[{}]{}`",
                &server_name,
                &replace_formatting(line[33..].to_string())
            );

            // send the message to the chat bridge channel
            if let Err(why) = ChannelId(chat_id).say(&ctx.http, message).await {
                println!("Error sending message: {:?}", why);
            }
        }
    }

    // if the lines are under 2k, we don't need to replace the file since it doesn't take much time
    // to process in the first place
    if lines < 2000 {
        return cur_line;
    }

    // if it is above 2k however, we can reset the pipe and notify the to the console
    gen_pipe(server_name, true).await;
    println!("*info: pipe file reset");

    // return new line count to update the one in the main file
    cur_line
}
