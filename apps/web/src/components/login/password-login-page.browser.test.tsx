import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { authProvider } from "@/auth-provider";
import { dataProvider } from "@/data-provider";
import { PasswordLoginPage } from "@/components/login/password-login-page";

describe("PasswordLoginPage", () => {
  it("shows a password field and no username/email field", async () => {
    const screen = render(
      <AdminContext authProvider={authProvider} dataProvider={dataProvider}>
        <PasswordLoginPage />
      </AdminContext>,
    );
    await expect.element(screen.getByLabelText(/password/i)).toBeVisible();
    expect(screen.container.querySelector('input[name="email"]')).toBeNull();
    expect(screen.container.querySelector('input[name="username"]')).toBeNull();
  });
});
