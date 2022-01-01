use serde_derive::Deserialize;
use std::fs;

use hypnos_core::*;

// main config struct, this would be used for child nodes of the bot, aka those that only do
// limited functions
//
// that distinction is going to be elaborated on further on in development
#[derive(Deserialize)]
pub struct Config {
    pub token: String,
    pub prefix: String,
    pub shell_access: Vec<u64>,
    pub build_dir: String,
    pub optional: Optional,
}

// define the optional fields, you could describe these as the 'interesting' ones, where all the
// action takes place
//
// these are used for the log file and tmux pipping system stuff
#[derive(Deserialize)]
pub struct Optional {
    pub chat_bridge_id: Option<u64>,
    pub server_name: Vec<String>,
    pub generic_name: Vec<String>,
    pub backup_time: Option<i64>,
    pub backup_dir: Option<String>,
    pub keep_time: Option<u64>,
    pub backup_store: Option<String>,
    pub ign_prefix: Option<String>,
}

/*
 * Loads the main config values into a struct that is passed back into
 * the main.rs file
 */
pub fn load_config(conf: String) -> Config {
    // check if the config exists, and warn if it's not absolute
    //
    // default path is non absolute, so ignore that warning if you don't care
    let _ = !check_dir(conf.to_owned(), true); 

    /*
     * TODO
     * for future values, if None, set to default
     *
     * use serde to parse and load the config into the struct
     */
    let core_config: Config =
        toml::from_str(&fs::read_to_string(conf).expect("*error: no config file found!"))
            .expect("*error: invalid config! please check it");

    // check if the build directory exists, this is where we expect the bot to rebuild itself if
    // needed
    if !check_dir(core_config.build_dir.to_owned(), false) {
        eprintln!("*error: invalid build dir");
    }

    // if the token field is empty then we can just exit, in the future however the backup system
    // and some other managment commands should still be accessable if the type is set to child
    // node; but this is for later in the future
    if Some(&core_config.token) == None {
        eprintln!("*error: no token found\n*fatal: exiting");
        std::process::exit(1);
    }

    // if the backup directory is defined but does not exists on the system, then trigger an error
    // and exit
    if core_config.optional.backup_dir != None
        && !check_dir(core_config.optional.backup_dir.to_owned().unwrap(), false)
    {
        eprintln!("*error: backup directory does not exist\n*fatal: exiting");
        std::process::exit(1);
    }

    // same as the backup directory for the directory backups are sent to
    if core_config.optional.backup_store != None
        && !check_dir(core_config.optional.backup_store.to_owned().unwrap(), false)
    {
        eprintln!("*error: backup store directory does not exist\n*fatal: exiting");
        std::process::exit(1);
    }

    // return the config to the main file to be used for awesome things ofc
    core_config
}
