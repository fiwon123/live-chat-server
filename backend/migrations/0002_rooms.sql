-- Chat rooms; the creator becomes the first member.
-- ON DELETE RESTRICT: deleting a user must not silently destroy rooms (and,
-- transitively, every message in them). No user-deletion feature exists yet.
CREATE TABLE rooms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_rooms_created_by ON rooms(created_by);