INSERT INTO users (
    username,
    email,
    display_name,
    password_hash,
    password_changed_at,
    role
)
SELECT
    'admin',
    'admin@example.com',
    'Admin',
    '$argon2id$v=19$m=19456,t=2,p=1$AorfSurLxJEiJe7vWSmMRA$FgyltbMBO/UeX1BAJcO/9e4j1y3LombFNpE1sYraSXI',
    NOW(),
    'admin'
WHERE NOT EXISTS (
    SELECT 1
    FROM users
    WHERE id <> 1
);
