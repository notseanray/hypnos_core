use serenity::{model::channel::Message, prelude::*};

pub async fn sessions(ctx: Context, msg: Message, servers: Vec<String>, generic: Vec<String>) {
    if msg.content.contains(&"ls") {
        let response = format!(
            "```MC sessions:\n{}\n\nGeneric sessions:\n{}```",
            servers.join("\n"),
            generic.join("\n")
        );
        if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
            println!("Error sending message: {:?}", why);
        }
    }
}
