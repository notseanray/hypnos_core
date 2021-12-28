use crate::*;
use serenity::model::channel::Message;

pub async fn syscheck(ctx: Context, msg: Message, chat_id: u64) {
    // Sending a message can fail, due to a network error, an
    // authentication error, or lack of permissions to post in the
    // channel, so log to stdout when some error happens, with a
    // description of it.
    sys_check(true, ctx, Some(msg), chat_id).await;
}
