//! Room database code
use anyhow::Result;
use matrix_sdk_base::{MinimalRoomMemberEvent, RoomInfo};
use ruma::{
    events::{
        presence::PresenceEvent,
        receipt::Receipt,
        room::member::{StrippedRoomMemberEvent, SyncRoomMemberEvent},
        AnyGlobalAccountDataEvent, AnyRoomAccountDataEvent, AnyStrippedStateEvent,
        AnySyncStateEvent, GlobalAccountDataEventType, RoomAccountDataEventType, StateEventType,
    },
    receipt::ReceiptType,
    serde::Raw,
    EventId, RoomId, UserId,
};
use sqlx::{
    database::HasArguments, types::Json, ColumnIndex, Database, Executor, IntoArguments, Row,
    Transaction,
};

use crate::{helpers::SqlType, StateStore, SupportedDatabase};

impl<DB: SupportedDatabase> StateStore<DB> {
    /// Deletes a room from the room store
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn remove_room(&self, room_id: &RoomId) -> Result<()>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'c> &'c mut <DB as sqlx::Database>::Connection: Executor<'c, Database = DB>,
        String: SqlType<DB>,
    {
        DB::room_remove_query()
            .bind(room_id.to_string())
            .execute(&*self.db)
            .await?;
        Ok(())
    }

    /// Sets global account data for an account data event
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn set_global_account_data<'c>(
        txn: &mut Transaction<'c, DB>,
        event_type: &GlobalAccountDataEventType,
        event_data: Raw<AnyGlobalAccountDataEvent>,
    ) -> Result<()>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'a> &'a mut Transaction<'c, DB>: Executor<'a, Database = DB>,
        String: SqlType<DB>,
        Option<String>: SqlType<DB>,
        Json<Raw<AnyGlobalAccountDataEvent>>: SqlType<DB>,
    {
        DB::account_data_upsert_query()
            .bind(None::<String>)
            .bind(event_type.to_string())
            .bind(Json(event_data))
            .execute(txn)
            .await?;

        Ok(())
    }

    /// Get global account data for an account data event type
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn get_account_data_event(
        &self,
        event_type: GlobalAccountDataEventType,
    ) -> Result<Option<Raw<AnyGlobalAccountDataEvent>>>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'c> &'c mut <DB as sqlx::Database>::Connection: Executor<'c, Database = DB>,
        String: SqlType<DB>,
        Option<String>: SqlType<DB>,
        Json<Raw<AnyGlobalAccountDataEvent>>: SqlType<DB>,
        for<'a> &'a str: ColumnIndex<<DB as Database>::Row>,
    {
        let row = DB::account_data_load_query()
            .bind(None::<String>)
            .bind(event_type.to_string())
            .fetch_optional(&*self.db)
            .await?;
        let row = if let Some(row) = row {
            row
        } else {
            return Ok(None);
        };
        let row: Json<Raw<AnyGlobalAccountDataEvent>> = row.try_get("account_data")?;
        Ok(Some(row.0))
    }

    /// Sets presence for a user
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn set_presence_event<'c>(
        txn: &mut Transaction<'c, DB>,
        user_id: &UserId,
        presence: Raw<PresenceEvent>,
    ) -> Result<()>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'a> &'a mut Transaction<'c, DB>: Executor<'a, Database = DB>,
        String: SqlType<DB>,
        Json<Raw<PresenceEvent>>: SqlType<DB>,
    {
        DB::presence_upsert_query()
            .bind(user_id.to_string())
            .bind(Json(presence))
            .execute(txn)
            .await?;
        Ok(())
    }

    /// Gets presence for a user
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn get_presence_event(&self, user_id: &UserId) -> Result<Option<Raw<PresenceEvent>>>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'c> &'c mut <DB as sqlx::Database>::Connection: Executor<'c, Database = DB>,
        String: SqlType<DB>,
        Json<Raw<PresenceEvent>>: SqlType<DB>,
        for<'a> &'a str: ColumnIndex<<DB as Database>::Row>,
    {
        let row = DB::presence_load_query()
            .bind(user_id.to_string())
            .fetch_optional(&*self.db)
            .await?;
        let row = if let Some(row) = row {
            row
        } else {
            return Ok(None);
        };
        let row: Json<Raw<PresenceEvent>> = row.try_get("presence")?;
        Ok(Some(row.0))
    }

    /// Stores room membership info for a user
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn set_room_membership<'c>(
        txn: &mut Transaction<'c, DB>,
        room_id: &RoomId,
        user_id: &UserId,
        member_event: SyncRoomMemberEvent,
    ) -> Result<()>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'a> &'a mut Transaction<'c, DB>: Executor<'a, Database = DB>,
        String: SqlType<DB>,
        Json<SyncRoomMemberEvent>: SqlType<DB>,
        bool: SqlType<DB>,
        Option<String>: SqlType<DB>,
    {
        let displayname = member_event
            .as_original()
            .and_then(|v| v.content.displayname.clone());
        DB::member_upsert_query()
            .bind(room_id.to_string())
            .bind(user_id.to_string())
            .bind(false)
            .bind(Json(member_event))
            .bind(displayname)
            .execute(txn)
            .await?;
        Ok(())
    }

    /// Stores stripped room membership info for a user
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn set_stripped_room_membership<'c>(
        txn: &mut Transaction<'c, DB>,
        room_id: &RoomId,
        user_id: &UserId,
        member_event: StrippedRoomMemberEvent,
    ) -> Result<()>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'a> &'a mut Transaction<'c, DB>: Executor<'a, Database = DB>,
        String: SqlType<DB>,
        Json<StrippedRoomMemberEvent>: SqlType<DB>,
        bool: SqlType<DB>,
        Option<String>: SqlType<DB>,
    {
        let displayname = member_event.content.displayname.clone();
        DB::member_upsert_query()
            .bind(room_id.to_string())
            .bind(user_id.to_string())
            .bind(true)
            .bind(Json(member_event))
            .bind(displayname)
            .execute(txn)
            .await?;
        Ok(())
    }

    /// Stores user profile in room
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn set_room_profile<'c>(
        txn: &mut Transaction<'c, DB>,
        room_id: &RoomId,
        user_id: &UserId,
        profile: MinimalRoomMemberEvent,
    ) -> Result<()>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'a> &'a mut Transaction<'c, DB>: Executor<'a, Database = DB>,
        String: SqlType<DB>,
        Json<MinimalRoomMemberEvent>: SqlType<DB>,
    {
        DB::member_profile_upsert_query()
            .bind(room_id.to_string())
            .bind(user_id.to_string())
            .bind(Json(profile))
            .execute(txn)
            .await?;
        Ok(())
    }

    /// Stores a state event for a room
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn set_room_state<'c>(
        txn: &mut Transaction<'c, DB>,
        room_id: &RoomId,
        event_type: &StateEventType,
        state_key: &str,
        state: Raw<AnySyncStateEvent>,
    ) -> Result<()>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'a> &'a mut Transaction<'c, DB>: Executor<'a, Database = DB>,
        String: SqlType<DB>,
        Json<Raw<AnySyncStateEvent>>: SqlType<DB>,
        bool: SqlType<DB>,
    {
        DB::state_upsert_query()
            .bind(room_id.to_string())
            .bind(event_type.to_string())
            .bind(state_key.to_owned())
            .bind(false)
            .bind(Json(state))
            .execute(txn)
            .await?;
        Ok(())
    }

    /// Stores a stripped state event for a room
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn set_stripped_room_state<'c>(
        txn: &mut Transaction<'c, DB>,
        room_id: &RoomId,
        event_type: &StateEventType,
        state_key: &str,
        state: Raw<AnyStrippedStateEvent>,
    ) -> Result<()>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'a> &'a mut Transaction<'c, DB>: Executor<'a, Database = DB>,
        String: SqlType<DB>,
        Json<Raw<AnyStrippedStateEvent>>: SqlType<DB>,
        bool: SqlType<DB>,
    {
        DB::state_upsert_query()
            .bind(room_id.to_string())
            .bind(event_type.to_string())
            .bind(state_key.to_owned())
            .bind(true)
            .bind(Json(state))
            .execute(txn)
            .await?;
        Ok(())
    }

    /// Stores account data for a room
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn set_room_account_data<'c>(
        txn: &mut Transaction<'c, DB>,
        room_id: &RoomId,
        event_type: &RoomAccountDataEventType,
        event_data: Raw<AnyRoomAccountDataEvent>,
    ) -> Result<()>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'a> &'a mut Transaction<'c, DB>: Executor<'a, Database = DB>,
        Option<String>: SqlType<DB>,
        String: SqlType<DB>,
        Json<Raw<AnyRoomAccountDataEvent>>: SqlType<DB>,
        bool: SqlType<DB>,
    {
        DB::account_data_upsert_query()
            .bind(Some(room_id.to_string()))
            .bind(event_type.to_string())
            .bind(Json(event_data))
            .execute(txn)
            .await?;
        Ok(())
    }

    /// Stores info for a room
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn set_room_info<'c>(
        txn: &mut Transaction<'c, DB>,
        room_id: &RoomId,
        room_info: RoomInfo,
    ) -> Result<()>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'a> &'a mut Transaction<'c, DB>: Executor<'a, Database = DB>,
        String: SqlType<DB>,
        Json<RoomInfo>: SqlType<DB>,
        bool: SqlType<DB>,
    {
        DB::room_upsert_query()
            .bind(room_id.to_string())
            .bind(false)
            .bind(Json(room_info))
            .execute(txn)
            .await?;
        Ok(())
    }

    /// Stores stripped info for a room
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn set_stripped_room_info<'c>(
        txn: &mut Transaction<'c, DB>,
        room_id: &RoomId,
        room_info: RoomInfo,
    ) -> Result<()>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'a> &'a mut Transaction<'c, DB>: Executor<'a, Database = DB>,
        String: SqlType<DB>,
        Json<RoomInfo>: SqlType<DB>,
        bool: SqlType<DB>,
    {
        DB::room_upsert_query()
            .bind(room_id.to_string())
            .bind(true)
            .bind(Json(room_info))
            .execute(txn)
            .await?;
        Ok(())
    }

    /// Stores receipt for an event
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn set_receipt<'c>(
        txn: &mut Transaction<'c, DB>,
        room_id: &RoomId,
        event_id: &EventId,
        receipt_type: &ReceiptType,
        user_id: &UserId,
        receipt: Receipt,
    ) -> Result<()>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'a> &'a mut Transaction<'c, DB>: Executor<'a, Database = DB>,
        String: SqlType<DB>,
        Json<Receipt>: SqlType<DB>,
    {
        DB::receipt_upsert_query()
            .bind(room_id.to_string())
            .bind(event_id.to_string())
            .bind(receipt_type.to_string())
            .bind(user_id.to_string())
            .bind(Json(receipt))
            .execute(txn)
            .await?;
        Ok(())
    }

    /// Retrieves a state event in room by event type and state key
    ///
    /// # Errors
    /// This function will return an error if the the query fails
    pub async fn get_state_event(
        &self,
        room_id: &RoomId,
        event_type: StateEventType,
        state_key: &str,
    ) -> Result<Option<Raw<AnySyncStateEvent>>>
    where
        for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
        for<'c> &'c mut <DB as sqlx::Database>::Connection: Executor<'c, Database = DB>,
        String: SqlType<DB>,
        Json<Raw<AnySyncStateEvent>>: SqlType<DB>,
        for<'a> &'a str: ColumnIndex<<DB as Database>::Row>,
    {
        let row = DB::state_load_query()
            .bind(room_id.to_string())
            .bind(event_type.to_string())
            .bind(state_key.to_owned())
            .fetch_optional(&*self.db)
            .await?;
        let row = if let Some(row) = row {
            row
        } else {
            return Ok(None);
        };
        let row: Json<Raw<AnySyncStateEvent>> = row.try_get("state_event")?;
        Ok(Some(row.0))
    }
}
