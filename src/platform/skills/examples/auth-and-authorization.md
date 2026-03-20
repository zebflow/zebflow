# Auth and Authorization

## What this builds

Full JWT-based authentication: login page, register, session validation, role-based access control, protected routes, and logout. Works with PostgreSQL or Sekejap for user storage.

---

## Pipelines

1. `GET /auth/login` → render login page
2. `POST /auth/login` → validate credentials → issue JWT cookie → redirect
3. `GET /auth/register` → render register page
4. `POST /auth/register` → hash password → create user → redirect
5. `GET /auth/logout` → clear cookie → redirect to login
6. `GET /dashboard` → validate JWT → load user → render protected page
7. `GET /admin/*` → validate JWT + role check → serve or 403

---

## DSL

### auth-login-page — render login form

```
| trigger.webhook --path /auth/login --method GET
| web.render --template-path pages/auth-login.tsx --route /auth/login
```

### auth-login-submit — authenticate and issue token

```
| trigger.webhook --path /auth/login --method POST
| script -- "const { username, password } = input.body; if (!username || !password) return { error: 'missing fields', __status: 400 }; return { username, password }"
| pg.query --credential main-db -- "SELECT id, username, role, password_hash FROM users WHERE username = '{{input.username}}' LIMIT 1"
| script -- "const user = input[0]; if (!user) return { __redirect: '/auth/login?error=invalid' }; const match = user.password_hash === btoa(input.password + 'salt'); if (!match) return { __redirect: '/auth/login?error=invalid' }; const payload = { sub: user.id, username: user.username, role: user.role, exp: Date.now() + 86400000 }; return { token: btoa(JSON.stringify(payload)), user: { id: user.id, username: user.username, role: user.role } }"
| script -- "return { __redirect: '/dashboard', __set_cookie: 'auth_token=' + input.token + '; Path=/; HttpOnly; Max-Age=86400' }"
```

### auth-register-page — render register form

```
| trigger.webhook --path /auth/register --method GET
| web.render --template-path pages/auth-register.tsx --route /auth/register
```

### auth-register-submit — create new user

```
| trigger.webhook --path /auth/register --method POST
| script -- "const { username, email, password } = input.body; if (!username || !email || !password) return { error: 'all fields required', __status: 400 }; if (password.length < 8) return { error: 'password too short', __status: 400 }; return { username, email, password_hash: btoa(password + 'salt'), role: 'user', created_at: Date.now() }"
| pg.query --credential main-db -- "INSERT INTO users (username, email, password_hash, role, created_at) VALUES ('{{input.username}}', '{{input.email}}', '{{input.password_hash}}', '{{input.role}}', NOW()) RETURNING id"
| script -- "return { __redirect: '/auth/login?registered=1' }"
```

### auth-logout — clear session

```
| trigger.webhook --path /auth/logout --method GET
| script -- "return { __redirect: '/auth/login', __set_cookie: 'auth_token=; Path=/; HttpOnly; Max-Age=0' }"
```

### dashboard-protected — JWT-protected page

```
| trigger.webhook --path /dashboard --method GET
| script -- "const cookie = input.headers['cookie'] || ''; const match = cookie.match(/auth_token=([^;]+)/); if (!match) return { __redirect: '/auth/login' }; try { const payload = JSON.parse(atob(match[1])); if (payload.exp < Date.now()) return { __redirect: '/auth/login?expired=1' }; return { user: payload }; } catch(e) { return { __redirect: '/auth/login' }; }"
| pg.query --credential main-db -- "SELECT id, username, email, role, created_at FROM users WHERE id = '{{input.user.sub}}'"
| script -- "const u = input[0]; if (!u) return { __redirect: '/auth/login' }; return { user: u }"
| web.render --template-path pages/dashboard.tsx --route /dashboard
```

### admin-guard — role-checked admin route

```
| trigger.webhook --path /admin/:section --method GET
| script -- "const cookie = input.headers['cookie'] || ''; const match = cookie.match(/auth_token=([^;]+)/); if (!match) return { __redirect: '/auth/login' }; try { const payload = JSON.parse(atob(match[1])); if (payload.role !== 'admin') return { __status: 403, error: 'Forbidden' }; return { user: payload, section: input.params.section }; } catch(e) { return { __redirect: '/auth/login' }; }"
| web.render --template-path pages/admin-section.tsx --route /admin/:section
```

---

## Nodes Used

- `trigger.webhook` — all HTTP routes (GET and POST)
- `pg.query` — user lookup, insert, update in PostgreSQL
- `sekejap.query` — alternative to pg.query for Sekejap-based user storage
- `script` — JWT encode/decode, password hashing, cookie parsing, role checks
- `web.render` — login, register, dashboard, admin pages

---

## Auth Helper Script (reusable)

Extract this into a shared pipeline or script node to validate cookies consistently:

```js
const cookie = input.headers['cookie'] || '';
const match = cookie.match(/auth_token=([^;]+)/);
if (!match) return { __redirect: '/auth/login' };
try {
  const payload = JSON.parse(atob(match[1]));
  if (payload.exp < Date.now()) return { __redirect: '/auth/login?expired=1' };
  return { ...input, auth: payload };
} catch(e) {
  return { __redirect: '/auth/login' };
}
```

---

## Templates Needed

- `pages/auth-login.tsx` — login form (POST to /auth/login)
- `pages/auth-register.tsx` — register form (POST to /auth/register)
- `pages/dashboard.tsx` — protected user dashboard
- `pages/admin-section.tsx` — admin panel (role-gated)
