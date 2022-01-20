use crate::*;
use serenity::model::channel::Message;

pub async fn execute(
    ctx: Context,
    msg: Message,
    ids: Vec<u64>,
    servers: Vec<String>,
    generic: Vec<String>,
) {
    let mut response: String = "invalid permissions".to_string();

    // check if the user is aurthorized
    if ids.contains(msg.author.id.as_u64()) {
        // determine the contents of the command
        let mut cmd_contents: Vec<&str> = msg.content.split(" ").collect();

        // remove the first word, as it is just the command name
        cmd_contents.remove(0);

        // specify the session target
        let server_target: String = cmd_contents[0].to_string();

        // remove the target from the command so we only have the command left
        cmd_contents.remove(0);

        // reform the command from the vector
        let cmd: String = cmd_contents.join(" ");

        if !servers.contains(&server_target) && !generic.contains(&server_target) {
            if let Err(why) = msg
                .channel_id
                .say(&ctx.http, "session does not exist!")
                .await
            {
                println!("Error sending message: {:?}", why);
            }
            return;
        }
        // send the command to the session
        send_command(server_target, cmd).await;

        // update the response
        response = "command sent to session".to_string();
    }
    // Sending a message can fail, due to a network error, an
    // authentication error, or lack of permissions to post in the
    // channel, so log to stdout when some error happens, with a
    // description of it.
    if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
        println!("Error sending message: {:?}", why);
    }
}
