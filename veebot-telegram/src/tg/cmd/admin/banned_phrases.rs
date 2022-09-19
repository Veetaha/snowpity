use crate::tg::{self, cmd, Bot};
use crate::util::prelude::*;
use crate::{db, err_val, Result, UserError};
use async_trait::async_trait;
use lazy_regex::regex_is_match;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;

pub(crate) async fn add_exceptional_phrase(ctx: &tg::Ctx, msg: &Message, phrase: String) -> Result {
    todo!()
}
pub(crate) async fn add_banned_phrase(ctx: &tg::Ctx, msg: &Message, phrase: String) -> Result {
    let created_by = msg.from().unwrap().id;

    governor::Quota::allow_burst(self, max_burst)

    // ctx.db.tg_chat_banned_phrases
    //     .create(msg.chat.id, &phrase, created_by)
    //     .await?;

    ctx.bot.reply_chunked(&msg, "Слово успешно добавлено в список запрещённых").await?;

    todo!()
}


pub(crate) async fn list_exceptional_phrases(ctx: &tg::Ctx, msg: &Message) -> Result {
    todo!()
}
pub(crate) async fn list_banned_phrases(ctx: &tg::Ctx, msg: &Message) -> Result {
    // let banned_phrases: Vec<_> = repo
    //     .tg_chat_banned_words
    //     .get_all_by_chat_id(msg.chat.id)
    //     .try_collect()
    //     .await?;

    // let futs = banned_words
    //     .iter()
    //     .map(|pattern| pattern.created_by)
    //     .collect::<HashSet<_>>()
    //     .into_iter()
    //     .map(|user_id| bot.get_chat_member(msg.chat.id, user_id));

    // let users = future::try_join_all(futs)
    //     .await?
    //     .into_iter()
    //     .map(|member| (member.user.id, member.user.full_name()))
    //     .collect::<HashMap<_, _>>();

    // let reply_msg = format!(
    //     "The following patterns are banned in this chat:\n{}",
    //     banned_words.iter().format_with("\n", |pattern, f| {
    //         let regex = teloxide::utils::markdown::code_inline(pattern.pattern.as_str());

    //         let creator = markdown::escape(&users.get(&pattern.created_by).unwrap());

    //         let creation_time_ago = util::time_ago_from_now(pattern.created_at);

    //         f(&format_args!(
    //             "{regex} \\(created by {creator} {creation_time_ago}\\)"
    //         ))
    //     })
    // );
    Ok(())
}

pub(crate) async fn delete_exceptional_phrase(ctx: &tg::Ctx, msg: &Message, phrase: String) -> Result {
    todo!()
}
pub(crate) async fn delete_banned_phrase(ctx: &tg::Ctx, msg: &Message, phrase: String) -> Result {
    // repo.tg_chat_banned_patterns
    //     .delete(msg.chat.id, &pattern)
    //     .await?;

    // let pattern = markdown::code_inline(pattern.as_str());

    // let reply_msg =
    //     format!("The pattern {pattern} was successfully removed from blacklist");

    // bot.reply_chunked(&msg, reply_msg).await?;
    todo!()
}

// pub(crate) async fn ban_regex(ctx: _, msg: Message, regex: &str) -> _ {
//     let pattern = regex::RegexBuilder::new()
//         ;regex::Regex::new()

//         Regex::new(&input).map_err(err_ctx!(UserError::InvalidRegex { input }))?;

//     let created_by = msg.kind.unwrap_as_common().from.as_ref().unwrap().id;

//     repo.tg_chat_banned_patterns
//         .create(msg.chat.id, &pattern, created_by)
//         .await?;

//     bot.reply_chunked(&msg, "The pattern was successfully added to blacklist")
//         .await?;
// }
//
//
// Cmd::BannedPatterns => {
//     let banned_patterns: Vec<_> = repo
//         .tg_chat_banned_patterns
//         .get_all_by_chat_id(msg.chat.id)
//         .try_collect()
//         .await?;

//     let futs = banned_patterns
//         .iter()
//         .map(|pattern| pattern.created_by)
//         .collect::<HashSet<_>>()
//         .into_iter()
//         .map(|user_id| bot.get_chat_member(msg.chat.id, user_id));

//     let users = future::try_join_all(futs)
//         .await?
//         .into_iter()
//         .map(|member| (member.user.id, member.user.full_name()))
//         .collect::<HashMap<_, _>>();

//     let reply_msg = format!(
//         "The following patterns are banned in this chat:\n{}",
//         banned_patterns.iter().format_with("\n", |pattern, f| {
//             let regex = teloxide::utils::markdown::code_inline(pattern.pattern.as_str());

//             let creator = markdown::escape(&users.get(&pattern.created_by).unwrap());

//             let creation_time_ago = util::time_ago_from_now(pattern.created_at);

//             f(&format_args!(
//                 "{regex} \\(created by {creator} {creation_time_ago}\\)"
//             ))
//         })
//     );

//     bot.reply_chunked(&msg, reply_msg).await?;
// }
// Cmd::UnbanPattern(input) => {
//     let pattern =
//         Regex::new(&input).map_err(err_ctx!(UserError::InvalidRegex { input }))?;

//     repo.tg_chat_banned_patterns
//         .delete(msg.chat.id, &pattern)
//         .await?;

//     let pattern = markdown::code_inline(pattern.as_str());

//     let reply_msg =
//         format!("The pattern {pattern} was successfully removed from blacklist");

//     bot.reply_chunked(&msg, reply_msg).await?;
// }
