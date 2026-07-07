import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { MemoryRouter } from "react-router";
import { Form } from "shadmin-core";
import { HeadersInput } from "@/resources/webhook/headers-input";

describe("HeadersInput", () => {
  it("renders existing headers from the map as editable rows", async () => {
    const screen = render(
      <MemoryRouter>
        <Form defaultValues={{ headers: { "x-golbat-secret": "abc" } }} onSubmit={() => undefined}>
          <HeadersInput />
        </Form>
      </MemoryRouter>,
    );
    const nameInput = screen.getByLabelText("Header");
    const valueInput = screen.getByLabelText("Value");
    await expect.element(nameInput).toBeVisible();
    await expect.element(valueInput).toBeVisible();
    expect((nameInput.element() as HTMLInputElement).value).toBe("x-golbat-secret");
    expect((valueInput.element() as HTMLInputElement).value).toBe("abc");
  });
});
