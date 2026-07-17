import type { AuthProvider } from "ra-core";

/** Demo auth: there's no server and no session — every check resolves, and the
 *  user is always a full admin. Mirrors the live `AuthProvider` surface so the
 *  shadmin shell wires up identically in both modes. */
export const authProvider: AuthProvider = {
  async login() {
    // No credentials in the demo — accept and move on.
  },
  async logout() {
    // Nothing to tear down.
  },
  async checkAuth() {
    // Always authenticated.
  },
  async checkError() {
    // No transient auth errors in a serverless demo.
  },
  async getPermissions() {
    return "admin";
  },
  async canAccess() {
    return true;
  },
};
