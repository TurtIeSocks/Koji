import type { AuthProvider } from "ra-core";
import { internalFetch } from "@/lib/http";

export const authProvider: AuthProvider = {
  async login(params) {
    const { password } = params as { password: string };
    const { status } = await internalFetch("/auth/login", {
      method: "POST",
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
