use super::model::*;
use crate::prelude::*;
use crate::Result;
use sea_query::{Expr, OnConflict};
use sqlx_bat::prelude::*;
use teloxide::types::{Chat, User};

pub(crate) struct TgChatQuery<'a> {
    pub(crate) chat: &'a Chat,
    pub(crate) requested_by: &'a User,
    pub(crate) action: TgChatAction,
}

pub(crate) struct TgChatRepo {
    db: sqlx::PgPool,
}

impl TgChatRepo {
    pub(crate) fn new(db: sqlx::PgPool) -> Self {
        Self { db }
    }

    #[metered_db]
    pub(crate) async fn register_chat(&self, ctx: TgChatQuery<'_>) -> Result {
        ctx.insert_statement()?
            .into_sqlx()
            .query()
            .execute(&self.db)
            .await?;

        Ok(())
    }

    #[metered_db]
    pub(crate) async fn get_chat(&self, ctx: TgChatQuery<'_>) -> Result<TgChat> {
        ctx.insert_statement()?
            .returning_all()
            .into_sqlx()
            .query_as::<TgChatRecord>()
            .fetch_one(&self.db)
            .await?
            .try_into_app()
            .map_err(Into::into)
    }

    #[metered_db]
    pub(crate) async fn get_or_update_captcha(&self, ctx: TgChatQuery<'_>) -> Result<bool> {
        ctx.insert_statement()?
            .returning_col(TgChatIden::IsCaptchaEnabled)
            .into_sqlx()
            .query_scalar()
            .fetch_one(&self.db)
            .await
            .map_err(Into::into)
    }
}

/// It is not guaranteed that the chat record exists in the database,
/// because we don't have an explicit action of registering the chat
/// in the database. We must lazily insert it if it doesn't exist.
impl<'a> TgChatQuery<'a> {
    fn insert_statement(self) -> Result<sea_query::InsertStatement> {
        let mut insert = sea_query::Query::insert();

        let mut columns = vec![
            TgChatIden::Id,
            TgChatIden::Kind,
            TgChatIden::Title,
            TgChatIden::Name,
            TgChatIden::InviteLink,
            TgChatIden::RegisteredByUserId,
            TgChatIden::RegisteredByUserName,
            TgChatIden::RegisteredByUserFullName,
            TgChatIden::RegisteredByAction,
        ];

        let values = sqlx_bat::simple_expr_vec![
            self.chat.id.try_into_db()?,
            TgChatKind::from_tg_api(self.chat).try_into_db()?,
            self.chat.title(),
            self.chat.username(),
            self.chat.invite_link(),
            self.requested_by.id.try_into_db()?,
            self.requested_by.username.clone(),
            self.requested_by.full_name(),
            self.action.try_into_db()?,
        ];

        let mut on_conflict = OnConflict::column(TgChatIden::Id);
        on_conflict.do_nothing();

        if self.action == TgChatAction::ToggleCaptchaCommand {
            columns.push(TgChatIden::IsCaptchaEnabled);
            on_conflict.value(
                TgChatIden::IsCaptchaEnabled,
                Expr::col(TgChatIden::IsCaptchaEnabled).not(),
            );
        }

        insert
            .into_table(TgChatIden::Table)
            .columns(columns)
            .values_panic(values)
            .on_conflict(on_conflict);
        Ok(insert)
    }
}
