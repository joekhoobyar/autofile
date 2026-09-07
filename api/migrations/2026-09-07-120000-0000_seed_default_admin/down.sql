DELETE FROM users
WHERE username = 'admin'
  AND email = 'admin@example.com'
  AND password_hash = '$argon2id$v=19$m=19456,t=2,p=1$AorfSurLxJEiJe7vWSmMRA$FgyltbMBO/UeX1BAJcO/9e4j1y3LombFNpE1sYraSXI'
  AND id <> 1;
