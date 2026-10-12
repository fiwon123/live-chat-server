-- Message reactions: one row per (message, user, emoji). Read off the message
-- and broadcast alongside it, so counts and "who reacted" stay consistent
-- across reconnects. Both FKs cascade — unlike messages, a reaction is cheap
-- and disposable, so it goes away with its message or its author.
CREATE TABLE message_reactions (
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (message_id, user_id, emoji)
);
