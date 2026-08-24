# Create the First User

After the API is running, create an initial user through the registration endpoint.

```bash
curl -i -X POST "http://localhost:8000/api/v1/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin@example.com",
    "email": "admin@example.com",
    "display_name": "Admin",
    "password": "changeme123"
  }'
```

Then open the UI at `http://localhost:5173` and sign in with the new account.

!!! warning "Use a stronger password"
    The example password is only for local development. Use a strong password for any shared or persistent environment.
