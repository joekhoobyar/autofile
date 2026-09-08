CREATE TABLE app_settings (
    id BIGINT PRIMARY KEY DEFAULT 1,
    allow_user_registration BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT app_settings_single_row CHECK (id = 1)
);

INSERT INTO app_settings (id, allow_user_registration)
VALUES (1, true)
ON CONFLICT (id) DO NOTHING;
