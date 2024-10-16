use std::fmt::Display;

use matrix_sdk::ruma::{OwnedRoomId, OwnedUserId};

#[derive(Clone)]
pub(crate) struct UserRoomId {
    pub(crate) user_id: OwnedUserId,
    pub(crate) room_id: OwnedRoomId,
}

impl Display for UserRoomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.user_id, self.room_id)
    }
}
