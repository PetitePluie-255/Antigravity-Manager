# Antigravity Standalone Refactoring Walkthrough

## Completed Work

### 1. Architecture Split

- **Backend**: Renamed `src-tauri` to `server`. Removed Tauri dependencies. Converted to strict Axum web server.
- **Frontend**: Created `web` directory. Removed Tauri dependencies. Converted to standard Vite + React SPA.

### 2. Backend (Server)

- **Framework**: Axum + Tokio.
- **API**: Re-implemented business APIs under `src/api/`.
  - `api/account.rs`: CRUD, Quota, Import (JSON only).
  - `api/proxy.rs`: Proxy start/stop, status, logs.
  - `api/logs.rs`: Log retrieval.
- **State**: Centralized `AppState` using traits for decoupling.
- **Storage**: Reused `core` storage logic (file-based).

### 3. Frontend (Web)

- **Network**: Replaced `invoke` with `fetch` via `api/client.ts`.
- **Pages**: Refactored `Accounts` page to remove Tauri dialogs/events.
- **Components**: Rewrote `AddAccountDialog` to separate "JSON Import" and "Manual Token" logic, removing OAuth flows.
- **Services**: Simplified `accountService.ts` and `useAccountStore.ts`.

## Verification

### Automated Checks

- `cargo check --bin antigravity-server`: Passed (with warnings).
- `tsc` (Frontend): Passed.

### API Tests

- `curl http://localhost:3000/healthz` -> ok
- `curl http://localhost:3000/api/accounts` -> Returns account list.
- `curl http://localhost:3000/api/config` -> Returns config.

### Manual Verification Required

1. **Start Backend**:
   ```bash
   cd server
   cargo run --bin antigravity-server
   ```
2. **Start Frontend**:
   ```bash
   cd web
   npm install # if needed
   npm run dev
   ```
3. **Verify UI**:
   - Access `http://localhost:1420`.
   - Check if account list loads.
   - Try "Refresh All Quotas".
   - Try "Add Account -> Import JSON".
