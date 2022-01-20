use crate::*;
use serenity::{model::channel::Message, prelude::*};
use std::fs;

pub async fn script(ctx: Context, msg: Message, ids: Vec<u64>) {
    let args: Vec<&str> = msg.content.split(" ").collect();
    if ids.contains(msg.author.id.as_u64()) {
        if args.to_owned().contains(&"unlock") {
            if !check_dir("/tmp/HypnosCore-script.lock".to_string(), true) {
                if let Err(why) = msg
                    .channel_id
                    .say(
                        ctx.to_owned().http,
                        "script lock file does not exist! Skipping futher steps".to_string(),
                    )
                    .await
                {
                    println!("Error sending message: {:?}", why);
                }
                return;
            }

            fs::remove_file("/tmp/HypnosCore-script.lock")
                .expect("*error: failed to delete script lock file!");

            if let Err(why) = msg
                .channel_id
                .say(ctx.to_owned().http, "removed lock file")
                .await
            {
                println!("Error sending message: {:?}", why);
            }
            return;
        }

        if args.to_owned().contains(&"lock") {
            if check_dir("/tmp/HypnosCore-script.lock".to_string(), true) {
                if let Err(why) = msg
                    .channel_id
                    .say(
                        ctx.to_owned().http,
                        "script lock file already exist! Skipping futher steps".to_string(),
                    )
                    .await
                {
                    println!("Error sending message: {:?}", why);
                }
                return;
            }

            fs::File::create("/tmp/HypnosCore-script.lock")
                .expect("*error: failed to create script lock file!");

            if let Err(why) = msg
                .channel_id
                .say(ctx.to_owned().http, "created script lock file")
                .await
            {
                println!("Error sending message: {:?}", why);
            }
            return;
        }
    }

    if args.len() > 1 {
        return;
    }

    let response = format!(
        "script lock file: {}",
        check_dir("/tmp/HypnosCore-script.lock".to_string(), true)
    );

    // Sending a message can fail, due to a network error, an
    // authentication error, or lack of permissions to post in the
    // channel, so log to stdout when some error happens, with a
    // description of it.
    if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
        println!("Error sending message: {:?}", why);
    }
}
