use config::*;
use std::{
    env, fs,
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

mod config;
use hypnos_core::*;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready, id::GuildId},
    prelude::*,
};

use commands::*;

/*
 * These variables are mostly used for data that must be accessed over various locations, they
 * require the use of the unsafe keyword to access unfortunantely
 */
static mut LINES: Vec<usize> = Vec::new();
static mut SERVERS: Vec<String> = Vec::new();
static mut GENERIC_SERVERS: Vec<String> = Vec::new();
static mut GENERIC_LINES: Vec<usize> = Vec::new();
static mut LAST_LINE: Vec<String> = Vec::new();
static mut IGN_PREFIX: String = String::new();
static mut SELF_ID: u64 = 0;

struct Handler {
    is_loop_running: AtomicBool,
    backup_time: i64,
    backup_dir: String,
    backup_store: String,
    keep_time: u64,
    prefix: String,
    chat_bridge_id: u64,
    shell_access: Vec<u64>,
    build_dir: String,
    server_name: Vec<String>,
    generic_name: Vec<String>,
    script_update: u64,
}

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the message event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        // if chat bridge is defined in the config, check if the message is in the chat bridge
        // channel, if it starts with the the server we are currently operating on then skip it
        if msg.channel_id == self.chat_bridge_id {
            for i in self.server_name.clone() {
                let server_name_format: String = format!("[{}]", i.to_owned());

                if !msg.content.starts_with(&server_name_format) {
                    let mut message = String::new();
                    let mut self_msg: bool = false;
                    unsafe {
                        if msg.author.id == SELF_ID {
                            self_msg = true;
                            // generate the tellraw command
                            message = format!(
                                "tellraw @a {{ \"text\": \"{}\" }}",
                                &msg.content.replace(|c: char| !c.is_ascii(), "")
                            );
                        }
                    }
                    if !self_msg {
                        // generate the tellraw command
                        message = format!(
                            "tellraw @a {{ \"text\": \"[{}] {}\" }}",
                            msg.author.name,
                            msg.content.replace(|c: char| !c.is_ascii(), "")
                        );
                    }
                    // send it to the correct tmux session
                    send_command(i.to_owned(), message).await;
                }
            }
            for i in self.generic_name.clone() {
                let server_name_format: String = format!("[{}]", i.to_owned());
                if !msg.content.starts_with(&server_name_format) {
                    let mut message = String::new();

                    let mut self_msg: bool = false;

                    // check if the author is itself, then we can shorten what is said in chat
                    unsafe {
                        if msg.author.id == SELF_ID {
                            self_msg = true;

                            // terraria command format to paste in chat
                            message = format!("say {}", &msg.content);
                        }
                    }

                    if !self_msg {
                        // generate the tellraw command
                        message = format!("say [{}] {}", msg.author.name, msg.content);
                    }

                    // send it to the correct tmux session
                    send_command(i.to_owned(), message).await;
                }
            }
        }

        let msgc = msg.content.replace(|c: char| !c.is_ascii(), "");

        let mut cmd = msgc.as_str();

        if &msg.content.len() > &2
            && &msg.content[0..1] == "="
            && msg.channel_id == self.chat_bridge_id
        {
            run_calc(
                ctx.to_owned(),
                self.chat_bridge_id,
                msg.content[1..].to_string(),
            )
            .await;
        }

        if msg
            .content
            .replace(|c: char| !c.is_ascii(), "")
            .starts_with("[")
            && msg.content.contains("> =")
            && msg.channel_id == self.chat_bridge_id
        {
            let start: Option<usize> = msgc.find("> =");
            if (start.unwrap() + 2) < msg.content.len() && start != None {
                let spliced: &str = &msgc[(start.unwrap() + 2)..];
                run_calc(ctx.to_owned(), self.chat_bridge_id, spliced.to_string()).await;
            }
        }

        // if we know that the message is not from chat bridge we can still check to see if it has
        // the bot prefix, if it doesn't then we can just skip it
        let igncmd = format!("> {}", &self.prefix);
        if &cmd.len() < &2 || &cmd[0..1] != &self.prefix {
            return;
        }

        // we now have to match the message with the appropriate command, serenity has a built in
        // framework to do this but in the future I want to specifically enable and disable some
        // commands
        //
        // plus I don't know how to use serenity properly
        // match just the command
        if msgc.find(" ") != None {
            cmd = &msg.content[1..msg.content.find(" ").unwrap()];
        } else {
            cmd = &msgc[1..];
        }

        match cmd {
            "backup" => {
                backup::backup(
                    Some(ctx),
                    Some(msg),
                    Some(self.shell_access.to_owned()),
                    self.keep_time,
                    self.backup_dir.to_owned(),
                    self.backup_store.to_owned(),
                    self.backup_time.to_owned() as u64,
                )
                .await
            }
            "ping" => ping::ping(ctx, msg).await,
            "help" => help::help(ctx, msg).await,
            "recompile" => {
                recompile::recompile(
                    ctx,
                    msg,
                    self.shell_access.clone(),
                    self.build_dir.to_owned(),
                )
                .await
            }
            "execute" => {
                execute::execute(ctx, msg, self.shell_access.clone()).await;
            }
            "syscheck" => {
                syscheck::syscheck(ctx, msg, self.chat_bridge_id).await;
            }
            "script" => {
                script::script(ctx, msg, self.shell_access.clone()).await;
            }
            _ => invalid::invalid(ctx, msg).await,
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} connected", ready.user.name);
        unsafe {
            SELF_ID = ready.user.id.0;
        }
    }

    // once the shard is initialized properly a cache is formed, we will use this as an indicator
    // of when to start the async event loops
    //
    // one benefit of these loops is even if there is a panic the bot will continue to run just fine
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        println!("cache built successfully, starting process loops");

        let chat_id = self.chat_bridge_id;

        let keept = self.keep_time;

        let msg_ctx = ctx.clone();

        // this atomic boolean ensures that only one of each loop runs and that they run in order
        // correctly
        if !self.is_loop_running.load(Ordering::Relaxed) {
            // chat bridge loop, this handles scanning the tmux pipe file (and repiping it to
            // prevent the file from getting too long)
            // once there is a new line in the pipe file it scans it for a few edge cases
            tokio::spawn(async move {
                loop {
                    unsafe {
                        for (i, e) in SERVERS.iter().enumerate() {
                            // update the line count and process the log
                            LINES[i] = update_messages(
                                IGN_PREFIX.to_owned(),
                                e.to_owned(),
                                LINES[i],
                                ctx.to_owned(),
                                chat_id,
                            )
                            .await;
                        }

                        // update any other sessions that must be scanned by the bot
                        for (i, e) in GENERIC_SERVERS.iter().enumerate() {
                            let (server_name, line) = update_messages_generic(
                                e.to_owned(),
                                GENERIC_LINES[i],
                                ctx.to_owned(),
                                chat_id,
                                LAST_LINE[i].to_owned(),
                            )
                            .await;

                            GENERIC_LINES[i] = server_name;
                            LAST_LINE[i] = line;
                        }
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
                let bdir = self.backup_dir.to_owned();
                let bs = self.backup_store.to_owned();
                let bt = self.backup_time.to_owned() as u64;
                // create another async loop to run in parallel
                tokio::spawn(async move {
                    loop {
                        let _res = backup::backup(
                            None,
                            None,
                            None,
                            keept,
                            bdir.to_owned(),
                            bs.to_owned(),
                            bt.to_owned(),
                        )
                        .await;

                        tokio::time::sleep(Duration::from_secs(backup_cycle as u64)).await;
                    }
                });
            }

            // perform checks on the server every 5 minutes, this makes sure backups don't take up
            // too much room, prevents the cpu from getting pinned at 100%, and ram usage from being too high
            tokio::spawn(async move {
                loop {
                    sys_check(false, msg_ctx.to_owned(), None, chat_id).await;
                    // clear any zombies generated in the meantime
                    reap();
                    tokio::time::sleep(Duration::from_secs(300)).await;
                }
            });

            // script thread, this thread executes anything in the script file in the default
            // folde
            let stime = self.script_update;
            tokio::spawn(async move {
                // various checks for different directorys or files to see if they exists, if they
                // don't then we must create them
                if !check_dir("./default".to_string(), true) {
                    println!("*info: default folder not found, generating it");
                    fs::create_dir("./default")
                        .expect("*info: failed to create new default folder");
                }

                if !check_dir("./hypnos_core.conf".to_string(), true) {
                    println!("*warn: config file not found, copying it from default");
                    fs::copy(
                        "./build/hypnos_core/default/template.conf",
                        "./hypnos_core.conf",
                    )
                    .expect("failed to copy default config");
                }

                if check_dir("./default/scriptrc".to_string(), true) {
                    if !check_dir("./default/cache".to_string(), true) {
                        println!("*info: no cache file found, creating it");
                        fs::File::create("./default/cache")
                            .expect("*error: failed to create cache file");
                    }
                    loop {
                        if check_dir("/tmp/HypnosCore-script.lock".to_string(), true) {
                            println!("*warn: script lock file is in place");
                            return;
                        }
                        // different shells work here fine but bash is very portable and widely
                        // used, so may as well use it here
                        //
                        // the 'scriptrc' file is basically a cron job or a scheduled/looped
                        // script, this is relatively useless for most people since you can't have
                        // it controllable per a process
                        let _script_loop = Command::new("bash")
                            .arg("./default/scriptrc")
                            .status()
                            .expect("failed to execute script command");
                        // clear any zombies generated in the meantime
                        reap();
                        tokio::time::sleep(Duration::from_secs(stime)).await;
                    }
                }
            });

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

    // TODO - add loading default config
    // if there are no env args, skip it, otherwise check them for certain conditions
    if envargs.len() > 1 {
        match envargs[1].as_str() {
            "reset-cfg" => default_cfg(true),
            "c" => config_path = envargs[2].to_owned(),
            "help" => print_help(),
            _ => println!("*warn: invalid argument"),
        }
    }

    // load the config into a defined struct in config.rs with serde
    let config: Config = load_config(config_path);

    // set the intial line count
    let mut cur_line: Vec<usize> = Vec::new();

    let mut cur_generic: Vec<usize> = Vec::new();

    let mut server_name: Vec<String> = Vec::new();

    let mut lines: Vec<String> = Vec::new();

    // if server_name and chat_bridge_id are not none, continue to fill out values
    if config.optional.chat_bridge_id != None {
        server_name = config.optional.server_name.clone();

        for i in &server_name {
            // generate the tmux pipe, this takes a little bit of extra time
            gen_pipe(i.to_owned(), true).await;
        }

        for i in &config.optional.generic_name {
            // generate the tmux pipe, this takes a little bit of extra time
            gen_pipe(i.to_owned(), true).await;
        }

        // wait for tmux to create the pipe file
        tokio::time::sleep(Duration::from_millis(
            (config.optional.server_name.len() * 20) as u64,
        ))
        .await;

        // set the line count based on how many lines there are in the file
        for i in server_name {
            cur_line.push(set_lines(i.to_owned()));
            lines.push("".to_string());
        }
        for i in &config.optional.generic_name {
            cur_generic.push(set_lines(i.to_owned()));
        }
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
        SERVERS = config.optional.server_name.clone();
        GENERIC_SERVERS = config.optional.generic_name.clone();
        GENERIC_LINES = cur_generic;
        IGN_PREFIX = config.optional.ign_prefix.clone().unwrap();
        LAST_LINE = lines;
    }

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&config.token)
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
            backup_time: backup,
            backup_dir: config.optional.backup_dir.unwrap(),
            backup_store: config.optional.backup_store.unwrap(),
            keep_time: config.optional.keep_time.unwrap(),
            prefix: config.prefix,
            chat_bridge_id: config.optional.chat_bridge_id.unwrap(),
            shell_access: config.shell_access,
            build_dir: config.build_dir,
            server_name: config.optional.server_name,
            generic_name: config.optional.generic_name,
            script_update: config.optional.script_update.unwrap(),
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
