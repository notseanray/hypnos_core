use crate::*;
use serenity::model::channel::Message;
use std::{
    fs::{create_dir, metadata, remove_file, File},
    io::{BufRead, BufReader},
    process::Command,
};

/*
 * VERY EXPERIMENTAL
 *
 * I highly suggest this is not used, as this feature is inheriently unstable due to the nature of
 * attempting to recompile itself from source
 *
 * the goal here is to pull updates upstream from a git repo and update with no pain, if it works
 * correctly, this would be very very nice as an update is as simply as a command in discord
 */
pub async fn recompile(ctx: Context, msg: Message, ids: Vec<u64>, build_dir: String) {
    // generate default response to command, this will get changed as the compilation is
    // successful, if not will indicate the invalid perms
    let mut response: String = "invalid permissions".to_string();

    // check if the message author has correct permissions
    if ids.contains(msg.author.id.as_u64()) {

        let args: Vec<&str> = msg.content.split(" ").collect();

        // create new tmux session to compile in with the name recompile
        Command::new("tmux")
            .args(["new", "-s", "recompile"])
            .output()
            .expect("*error: failed to generate tmux session for recompilation");

        // go into the build directory specified in the config file
        let dir = format!("cd {}", build_dir);

        // send the cd command to the tmux session to get into the correct directory
        send_command("recompile".to_string(), dir).await;

        // check changelog of the active version of the bot to determine if it can
        // automatically restart
        let change_log =
            File::open("./changelog").expect("*error: failed to read changelog while recompiling");

        // open the old change log and count the number of lines
        let reader = BufReader::new(change_log);

        let changed_lines: usize = reader.lines().count();

        // if the backup option is selected, check if the directory exists and if it doesn't create
        // it
        //
        // next, copy the files into the backup directory
        if args.contains(&"backup") {
            let old_build: String = format!("{}/old_src", build_dir);
            if !check_dir(old_build.to_owned(), false) {
                println!("creating backup directory");
                create_dir(old_build.to_owned())
                    .expect("*error: failed to create backup folder for old source");
            }

            let copy_cmd: String = format!("cp -r {}/hypnos_core {}", build_dir, old_build);
            send_command("recompile".to_string(), copy_cmd).await;
        }

        // if git is one of the options, then clone the main repo
        // TODO
        // add separate repo as optional config option
        if args.contains(&"git") {
            let gitcmd = Command::new("git")
                .args(["clone", "https://github.com/NotCreative21/hypnos_core.git"])
                .status()
                .expect("*error: failed to git clone in attempt to recompile");
            if !gitcmd.success() {
                eprintln!("*error: failed to copy new files from github")
            }
        }

        let new_change_path = format!("{}/hypnos_core/changelog", build_dir);

        // check the new change log for new lines, if there are then print them out to the terminal
        let new_changes = File::open(new_change_path)
            .expect("*error: failed to read changelog while recompiling");

        let new_reader = BufReader::new(new_changes);

        let mut change: bool = false;

        for (i, line) in new_reader.lines().enumerate() {
            if i > changed_lines {
                let line = line.unwrap();

                // print the new changes out to the terminal
                println!("new change: {:#?}", line);

                // look for the keyword 'update', if it is found then we know that we should not
                // compile without changes to the config file first
                if line.contains("*update") {
                    change = true;
                    response =
                        "error: compilation aborted, changes to config must be applied".to_string();
                }
            }
        }

        // if there is no change requiring reconfig, then we can procede with compiling
        if !change {
            // send the command to go the new code and compile it with cargo in release mode
            send_command(
                "recompile".to_string(),
                "cd hypnos_core && cargo build --release".to_string(),
            )
            .await;

            // at this stage, change the response to include that compilation was at least
            // attempted
            response = "compilation was attempted, however, an error occured or the resulting binary is not 'new'".to_string();

            let result_bin = format!("{}/hypnos_core/target/release/hypnos_core", build_dir);

            let old_bin = format!("{}/../hypnos_core", build_dir);

            // if the resulting file has a timestamp that is greater than the old one (unix
            // timestamp format) then we know that something *might have* gone right, and we can
            // change the response and copy the new file into the folder
            //
            // TODO - check if this actually fucking works
            if metadata(result_bin.to_owned()).unwrap().modified().unwrap()
                < metadata(old_bin.to_owned()).unwrap().modified().unwrap()
            {
                response = "compilation successful".to_string();

                remove_file(old_bin).expect("failed to remove old binary");

                // send the copy command to overwrite the old file
                send_command(
                    "recompile".to_string(),
                    "cp target/release/hypnos_core ../..".to_string(),
                )
                .await;
            }
        }
    }
    // Sending a message can fail, due to a network error, an
    // authentication error, or lack of permissions to post in the
    // channel, so log to stdout when some error happens, with a
    // description of it.
    if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
        println!("Error sending message: {:?}", why);
    }
    reap();
}
