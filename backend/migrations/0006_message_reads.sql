-- Per-room read cursor: the newest message each member has marked as read.
-- Persisted so unread counts survive reconnects. `last_read_message_id` is
-- nullable: a member who has never opened the room has no cursor, so every
-- message counts as unread. FKs cascade with their room/user; deleting the
-- cursor message itself (there is no per-message delete today) would null the
-- cursor rather than orphan it.
CREATE TABLE message_reads (
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_read_message_id UUID REFERENCES messages(id) ON DELETE SET NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (room_id, user_id)
);

CREATE INDEX idx_message_reads_user ON message_reads(user_id);
