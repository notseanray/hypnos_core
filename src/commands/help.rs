use serenity::{model::channel::Message, prelude::*};

pub async fn help(ctx: Context, msg: Message) {
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
                           "
    );
    // Sending a message can fail, due to a network error, an
    // authentication error, or lack of permissions to post in the
    // channel, so log to stdout when some error happens, with a
    // description of it.
    if let Err(why) = msg.channel_id.say(&ctx.http, help_msg).await {
        println!("Error sending message: {:?}", why);
    }
}
