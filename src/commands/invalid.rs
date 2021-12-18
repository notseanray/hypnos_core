use serenity::{model::channel::Message, prelude::*};

pub async fn invalid(ctx: Context, msg: Message) {
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
