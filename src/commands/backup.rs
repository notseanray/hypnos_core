use crate::*;
use serenity::model::channel::Message;

// TODO
// create command structure
pub async fn backup(ctx: Context, msg: Message, ids: Vec<u64>) {
    // plan
    // lock file for scheduled backups && make sure temp ones cannot be made while lock is there
    //
    // backup priority list, allow 3 simultaneous backups, store at last number in folder name
    // compress scheduled backups
    // delete old ones

    if ids.contains(&!msg.author.id.as_u64()) {
        if let Err(why) = msg
            .channel_id
            .say(
                &ctx.http,
                "backup started",
            )
            .await
            {
                println!("Error sending message: {:?}", why);
        }
        return;
       
    }

    // Sending a message can fail, due to a network error, an
    // authentication error, or lack of permissions to post in the
    // channel, so log to stdout when some error happens, with a
    // description of it.
    if let Err(why) = msg
        .channel_id
        .say(
            &ctx.http,
            "Invalid command, please use help to get a list of commands",
        )
        .await
    {
        println!("Error sending message: {:?}", why);
    }
}
