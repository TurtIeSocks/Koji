import { describe, expect, it } from "vitest";
import { authProvider } from "@/auth-provider";

describe("authProvider", () => {
  it("logs in with a correct password", async () => {
    await expect(
      authProvider.login({ password: "correct-horse" }),
    ).resolves.toBeUndefined();
  });

  it("rejects a wrong password", async () => {
    await expect(
      authProvider.login({ password: "wrong" }),
    ).rejects.toBeTruthy();
  });

  it("checkAuth resolves only after login", async () => {
    await expect(authProvider.checkAuth({})).rejects.toBeTruthy();
    await authProvider.login({ password: "correct-horse" });
    await expect(authProvider.checkAuth({})).resolves.toBeUndefined();
  });

  it("checkError rejects on 401 and resolves otherwise", async () => {
    await expect(authProvider.checkError({ status: 401 })).rejects.toBeTruthy();
    await expect(authProvider.checkError({ status: 500 })).resolves.toBeUndefined();
  });

  it("grants all permissions (no RBAC)", async () => {
    await expect(authProvider.getPermissions!({})).resolves.toBe("admin");
    await expect(authProvider.canAccess!({ resource: "geofence", action: "edit" })).resolves.toBe(true);
  });
});
