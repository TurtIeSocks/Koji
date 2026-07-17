import type { AuthProvider } from "ra-core";
import { internalFetch } from "@/lib/http";

export const authProvider: AuthProvider = {
  async login(params) {
    const { password } = params as { password: string };
    // KOJI_SECRET doubles as (a) the Bearer token that clears `public_validator`
    // and (b) the password the login handler checks. A fresh cookie-only client
    // has no session yet, so the login request MUST carry the Bearer to get
    // *through* the gate; the handler then sets the session cookie that
    // authorizes every later request. Sending only the body → 401 at the gate.
    const { status } = await internalFetch("/auth/login", {
      method: "POST",
      headers: { Authorization: `Bearer ${password}` },
      body: JSON.stringify({ password }),
    });
    if (status >= 200 && status < 300) return;
    throw new Error("Invalid password");
  },

  async logout() {
    await internalFetch("/auth/logout", { method: "POST" });
  },

  async checkAuth() {
    const { status } = await internalFetch("/auth/me");
    if (status >= 200 && status < 300) return;
    throw new Error("Not authenticated");
  },

  async checkError(error) {
    const status = (error as { status?: number })?.status;
    if (status === 401 || status === 403) throw new Error("Session expired");
  },

  async getPermissions() {
    return "admin";
  },

  async canAccess() {
    return true;
  },
};
