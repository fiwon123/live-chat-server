-- Chat messages. Persisted BEFORE being broadcast over WebSockets, so clients
-- see consistent ids/timestamps after reconnects. The index supports the
-- `(room_id, created_at, id)` history-pagination order.
--
-- user_id is RESTRICT: removing a user must not punch holes in a room's shared
-- history. room_id cascades so deleting a room removes its messages.
CREATE TABLE messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_messages_room_created ON messages(room_id, created_at, id);
CREATE INDEX idx_messages_user ON messages(user_id);