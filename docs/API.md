# Dashboard & API Documentation 🌐

The web dashboard provides a management interface and real-time statistics.

## 🔑 Authentication
Authentication is handled via **Telegram Login Widget**. Upon successful callback, a signed cookie `user_session` is issued. 
> **Note**: In production, ensure `COOKIE_KEY` is set to a persistent secret.

## 📍 Endpoints

### 🏠 Web Interface
- `GET /`: Main dashboard (requires login). Shows stats and recent history.
- `GET /login`: Login page with Telegram widget.
- `GET /admin`: Admin panel (restricted to `ADMIN_ID`).

### ⚙️ Management (POST)
All management routes require an active session and redirect back to `/` on completion.
- `POST /dashboard/update`: Update global user settings (Mode, Language, AI toggle).
- `POST /dashboard/chat/toggle/{chat_id}`: Enable/disable bot in a specific group.
- `POST /dashboard/chat/mode/{chat_id}`: Set group mode (Reply/Delete).
- `POST /dashboard/custom_rule/add`: Add a new tracking parameter pattern.
- `POST /dashboard/custom_rule/delete/{id}`: Remove a custom rule.

### 📊 Data & Real-time
- `GET /events`: SSE (Server-Sent Events) stream for real-time dashboard updates.
- `GET /dashboard/export`: Download history as a CSV file.
- `GET /health`: Basic health check returning `OK`.

## 🛠️ Custom Logic
Custom rules are applied *before* the standard ClearURLs ruleset. Patterns are matched against URL query parameter keys.
