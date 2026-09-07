# Default Admin User

After the API runs migrations on a fresh installation, Autofile creates a default admin user when no non-system users exist.

```text
username: admin
email: admin@example.com
password: admin123!
```

Open the UI at `http://localhost:5173` and sign in with the default credentials.

!!! warning "Change the default password"
    The default admin must change this password before using other authenticated features.
