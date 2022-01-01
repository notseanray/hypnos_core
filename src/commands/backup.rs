use crate::*;
use serenity::model::channel::Message;
use chrono::prelude::*;
use std::fs;
use std::time::SystemTime;

pub async fn backup(ctx: Option<Context>, msg: Option<Message>, ids: Option<Vec<u64>>, keep_time: u64, backup_dir: String, backup_store: String, btime: u64) {
    
    if !check_dir(backup_dir.to_owned(), false) || !check_dir(backup_store.to_owned(), false) {
        eprintln!("*error: please check your config! either backup directory or backup store is invalid!");
        return;
    }
    
    // if it was used from the command we can execute what is needed
    if ctx.is_some() && msg.is_some() && ids != None {

        if !ids.unwrap().contains(&msg.to_owned().unwrap().author.id.as_u64()) {
            if let Err(why) = msg.unwrap()
                .channel_id
                .say(
                    &ctx.unwrap().http,
                    "invalid permissions to use backup system!",
                )
                .await
                {
                    println!("Error sending message: {:?}", why);
            }
            return;
           
        }

        let msg = msg.expect("failed to unwrap");

        let args: Vec<&str> = msg.content.split(" ").collect();

        if args.to_owned().contains(&"ls") {
            let mut backups: Vec<String> = Vec::new();
            let mut youngest = keep_time;
            for i in fs::read_dir(&backup_store).expect("failed to read backup dir!") {
                let i = i.ok().unwrap().path();
                if i.is_file() {
                    let name = i.to_owned().file_name().unwrap().to_string_lossy().into_owned();
                    let e = format!(
                        "{} ({:.2}G)", 
                        name[11..].to_string(), 
                        i.metadata().ok().unwrap().len() as f64 / 1000000000.0,
                    );

                    backups.push(e);

                    if let Ok(time) = &i.metadata().ok().unwrap().created() {
                        if SystemTime::now().duration_since(time.to_owned()).ok().unwrap().as_secs() < youngest {
                            youngest = SystemTime::now().duration_since(time.to_owned()).ok().unwrap().as_secs(); 
                        } 
                    }
                }
            }
            if backups.len() == 0 {
                if let Err(why) = msg
                    .channel_id
                    .say(
                        &ctx.unwrap().http,
                        "no backups to display!",
                    )
                    .await
                    {
                        println!("Error sending message: {:?}", why);
                }
                return;
            }

            let msg_res = format!(
                "```Backups are displayed in UTC time: (YYYY/DD/MM/HH/mm/SS)\n{}\nLast Backup: {}s ago Next Backup in: {}s```", 
                backups.join("\n"),
                &youngest,
                btime - &youngest,
            );

            if let Err(why) = msg
                .channel_id
                .say(
                    &ctx.to_owned().unwrap().http,
                    msg_res,
                )
                .await
                {
                    println!("Error sending message: {:?}", why);
            }
        }

        if args.to_owned().contains(&"rm") {
            let args = args.to_owned();

            let mut index = 0;

            for (i, e) in args.iter().enumerate() {
                if e == &"rm" {
                    index = i;
                }
            }

            if args.len() < index + 1 || args.len() < 3 {
                return;
            }

            let path = format!(
                "{}/HypnosCore-{}", 
                backup_store, 
                args[index + 1]
            );

            if !check_dir(path.to_owned(), true) {
                if let Err(why) = msg
                    .channel_id
                    .say(
                        &ctx.to_owned().unwrap().http,
                        "Location does not exists on file system",
                    )
                    .await
                    {
                        println!("Error sending message: {:?}", why);
                }
            }

            match fs::remove_file(path) {
                Ok(_) => {
                    let succ = format!("successfully removed backup: {}", args[index + 1]);
                    if let Err(why) = msg
                        .channel_id
                        .say(
                            &ctx.to_owned().unwrap().http,
                            succ,
                        )
                        .await
                        {
                            println!("Error sending message: {:?}", why);
                    }
                }
                Err(e) => {
                    let err = format!("failed to remove old backup due to: {:?}", e);
                    if let Err(why) = msg
                        .channel_id
                        .say(
                            ctx.to_owned().unwrap().http,
                            err, 
                        )
                        .await
                        {
                            println!("Error sending message: {:?}", why);
                    }
                }
            };

        }

        if args.to_owned().contains(&"new") {

            if let Err(why) = msg
                .channel_id
                .say(
                    ctx.to_owned().unwrap().http,
                    "Attempting to start new backup, this may take a while".to_string(),
                )
                .await
                {
                    println!("Error sending message: {:?}", why);
            }
            let res = new(backup_store.to_owned(), backup_dir.to_owned(), keep_time);
            if let Err(why) = msg
                .channel_id
                .say(
                    ctx.to_owned().unwrap().http,
                    res.to_owned(),
                )
                .await
                {
                    println!("Error sending message: {:?}", why);
            }
        }

        if args.to_owned().contains(&"clear-lock") {

            if !check_dir("/tmp/HypnosCore-Backup.lock".to_string(), true) {
                if let Err(why) = msg
                    .channel_id
                    .say(
                        ctx.to_owned().unwrap().http,
                        "Backup lock file does not exists! Skipping futher steps".to_string(),
                    )
                    .await
                    {
                        println!("Error sending message: {:?}", why);
                }
                return;
            }

            fs::remove_file("/tmp/HypnosCore-Backup.lock")
                .expect("*error: failed to delete lock file!");
            
            if let Err(why) = msg
                .channel_id
                .say(
                    ctx.to_owned().unwrap().http,
                    "removed lock file",
                )
                .await
                {
                    println!("Error sending message: {:?}", why);
            }
        }

        return;
    }

    new(backup_store.to_owned(), backup_dir.to_owned(), keep_time);
    
    // remove any zombie processes that were created during the process
    reap();
}

fn new(backup_store: String, backup_dir: String, keep_time: u64) -> String {
    
    // if it is not from the command, we know it's scheduled and therefore we can create a new
    // archive and store it
    let mut sys = System::new_all();
    sys.refresh_disks_list();
    sys.refresh_disks();

    // check the disk to ensure that a backup is safe
    let (u, t, i) = check_disk(&sys);

    // if there is an issue then we can just skip and continue
    if (i as u16 * 100) == 10 {
        eprintln!("*error: backup skipped due to low disk space on index: {}", i);
        return "Disk space is low! Skipping backup".to_string();
    }

    // if the lock file exists then something could be wrong, we will skip if this happens
    if check_dir("/tmp/HypnosCore-Backup.lock".to_string(), true) {
        eprintln!("*warn: backup lock file exists! skipping backup");
        return "Lock file exists, skipping backup".to_string();
    } 

    // create the lock file while we perform backups
    fs::File::create("/tmp/HypnosCore-Backup.lock").expect("failed to create lock file");

    // so we can keep track of how it takes to backup, we will measure it
    let btime = std::time::Instant::now();

    let root_res = format!("{}/root_copy/", &backup_store);

    // if the folder doesn't exists for the root copy, create it
    if !check_dir(root_res.to_owned(), false) {
        println!("*warn: failed to find root copy directory, creating it instead");
        fs::create_dir_all(root_res.to_owned())
            .expect("*error: failed to create root_cpy dir");
    }

    // copy the directory into the backup directory, it is called the 'root copy' because it is a
    // direct copy of the folder, we use the option -u to only copy the latest updates and ensure
    // that backing up is speedy
    let root_cpy = Command::new("cp")
        .args(["-ur", &backup_dir, &root_res])
        .status()
        .expect("*error: failed to update root copy");

    // notify of the output of the backup
    if root_cpy.success() {
        println!("*info: updated root copy in {}", &backup_store);
    }
    else {
        eprintln!("*error: failed to update root copy");
        return "failed to update root copy".to_string();
    }

    // name the new backup based on the time it was taken
    let backup_name = format!(
        "{}/HypnosCore-{}.tar.gz", 
        backup_store, Utc::now().to_string().replace(":", "_").replace(" ", "_")
    );

    let root_backup = format!("{}/root_copy", &backup_store);

    // create a tar archive of the backup
    let backupcmd = Command::new("tar")
        .args(["-czf", &backup_name, &root_backup])
        .status()
        .expect("failed to create backup");

    // if it was a success, we can remove copies older than the specified time, otherwise we can
    // continue
    if backupcmd.success() {
        println!("*info: creating new backup: {}, storage space usage: {:.2}%", backup_name, (u/t)*100.0);
        println!("*info: finished creating backup in time: {:#?}", btime.elapsed());
        for i in fs::read_dir(&backup_store).expect("failed to read backup dir!") {
            let i = i.ok().unwrap().path();
            if i.is_file() {
                if let Ok(time) = &i.metadata().ok().unwrap().created() {
                    if SystemTime::now().duration_since(time.to_owned()).ok().unwrap().as_secs() >= keep_time {
                        fs::remove_file(i).expect("*error: could not remove old backup");
                    } 
                }
            }
        }
        println!("*info: backup cycle complete");
    }
    else {
        eprintln!("*error: failed to create backup!");
        eprintln!("*info: due to this, skipping deletion of stored backups");
    }

    // remove the now uneeded lock file
    fs::remove_file("/tmp/HypnosCore-Backup.lock").expect("failed to remove backup lock file!");
    return "Successfully created new backup".to_string();
}
