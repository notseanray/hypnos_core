use config::*;
use std::{
    env,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

mod commands;
mod config;
use crate::commands::*;
use hypnos_core::*;

// this is my janky command system, each command is it's own function in the commands module
use help::help;
use invalid::invalid;
use ping::ping;
use recompile::recompile;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready, id::GuildId},
    prelude::*,
};

/*
 * These variables are mostly used for data that must be accessed over various locations, they
 * require the use of the unsafe keyword to access unfortunantely
 */
static mut LINES: usize = 0;
static mut SERVER_NAME: String = String::new();
static mut IGN_PREFIX: String = String::new();

struct Handler {
    is_loop_running: AtomicBool,
    backup_time: i64,
    prefix: String,
    chat_bridge_id: u64,
    shell_access: Vec<u64>,
    build_dir: String,
}

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        unsafe {
            let server_name_format = format!("`[{}]", &SERVER_NAME);

            // if chat bridge is defined in the config, check if the message is in the chat bridge
            // channel, if it starts with the the server we are currently operating on then skip it
            if Some(&self.chat_bridge_id) != None
                && msg.channel_id == self.chat_bridge_id
                && !msg.content.starts_with(&server_name_format)
            {
                // generate the tellraw command
                let message: String = format!(
                    "tellraw @a {{ \"text\": \"[{}] {}\" }}",
                    msg.author.name,
                    msg.content.replace("\n", "")
                );
                // send it to the correct tmux session
                send_command(SERVER_NAME.to_owned(), message).await;
            }
        }

        // if we know that the message is not from chat bridge we can still check to see if it has
        // the bot prefix, if it doesn't then we can just skip it
        if &msg.content[0..1] != &self.prefix {
            return;
        }

        // we now have to match the message with the appropriate command, serenity has a built in
        // framework to do this but in the future I want to specifically enable and disable some
        // commands
        //
        // plus I don't know how to use serenity properly
        match &msg.content[1..] {
            "ping" => ping(ctx, msg).await,
            "help" => help(ctx, msg).await,
            "recompile" => {
                recompile(
                    ctx,
                    msg,
                    self.shell_access.clone(),
                    self.build_dir.to_owned(),
                )
                .await
            }
            _ => invalid(ctx, msg).await,
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} connected", ready.user.name);
    }

    // once the shard is initialized properly a cache is formed, we will use this as an indicator
    // of when to start the async event loops
    //
    // one benefit of these loops is even if there is a panic the bot will continue to run just fine
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        println!("cache built successfully, starting process loops");

        let chat_id = self.chat_bridge_id;

        // this atomic boolean ensures that only one of each loop runs and that they run in order
        // correctly
        if !self.is_loop_running.load(Ordering::Relaxed) {
            // chat bridge loop, this handles scanning the tmux pipe file (and repiping it to
            // prevent the file from getting too long)
            // once there is a new line in the pipe file it scans it for a few edge cases
            tokio::spawn(async move {
                loop {
                    unsafe {
                        // update the line count and process the log
                        LINES = update_messages(
                            IGN_PREFIX.to_owned(),
                            SERVER_NAME.to_owned(),
                            LINES,
                            ctx.to_owned(),
                            chat_id,
                        )
                        .await;
                    }

                    // update every 4th of a second, this time can be changed to 500 without it
                    // being too noticable, any message sent over the past message will get sent
                    // regardless
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            });

            // backup manager loop, if the delay between backups is under 200 seconds we can just
            // skip it
            if self.backup_time > 200 {
                let backup_cycle: i64 = self.backup_time;
                // create another async loop to run in parallel
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(Duration::from_secs(backup_cycle as u64)).await;
                    }
                });
            }

            // Now that the loops are running, we can set the atomic bool to true
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }
}

#[tokio::main]
async fn main() {
    // create a startup timer to measure how long instigating a client and connecting the tmux
    // pipes takes, realistically this is a bit lower than what it prints because we must wait some
    // for tmux to generate the pipe file
    let startup = Instant::now();

    // collect arguments at startup
    let envargs: Vec<String> = env::args().collect();

    // set the default config path
    let mut config_path: String = "./hypnos_core.conf".to_string();

    // if there are no env args, skip it, otherwise check them for certain conditions
    if envargs.len() > 1 {
        match envargs[1].as_str() {
            "reset-cfg" => println!("coming soon"),
            "c" => config_path = envargs[2].to_owned(),
            "help" => print_help(),
            _ => println!("*warn: invalid argument"),
        }
    }

    // load the config into a defined struct in config.rs with serde
    let config: Config = load_config(config_path);

    // set the intial line count
    let mut cur_line: usize = 0;

    let mut server_name = String::new();

    // if server_name and chat_bridge_id are not none, continue to fill out values
    if config.optional.server_name != None && config.optional.chat_bridge_id != None {
        server_name = config.optional.server_name.unwrap();

        // generate the tmux pipe, this takes a little bit of extra time
        gen_pipe(server_name.to_owned(), true).await;

        // wait for tmux to create the pipe file
        tokio::time::sleep(Duration::from_millis(100)).await;

        // set the line count based on how many lines there are in the file
        cur_line = set_lines(server_name.to_owned());
    }

    let mut backup: i64 = -1;

    // check if the backup timer is set, if it is then set backup to that othewise leave it as -1
    // to signifiy that it won't be running
    if config.optional.backup_time != None && config.optional.backup_time.unwrap() > 200 {
        backup = config.optional.backup_time.unwrap();
    }

    // set those static varibles to their correct values, needs unsafe unfortunately but I am too
    // lazy to rewrite this in 100% mEmOrY sAfE aNd ThReAd sAfE rUsT
    //
    // just don't crash the bot and you won't have problems with memory or data races :)
    unsafe {
        LINES = cur_line;
        SERVER_NAME = server_name;
        IGN_PREFIX = config.optional.ign_prefix.unwrap();
    }

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&config.token)
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
            backup_time: backup,
            prefix: config.prefix,
            chat_bridge_id: config.optional.chat_bridge_id.unwrap(),
            shell_access: config.shell_access,
            build_dir: config.build_dir,
        })
        .await
        .expect("Err creating client");

    print!("client loaded in: {:#?}, ", startup.elapsed());

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
