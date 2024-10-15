use matrix_sdk::ruma::{OwnedRoomId, OwnedUserId};

#[derive(Clone)]
pub(crate) struct UserRoomId {
    pub(crate) user_id: OwnedUserId,
    pub(crate) room_id: OwnedRoomId,
}
