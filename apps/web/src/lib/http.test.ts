import { describe, expect, it } from "vitest";
import { unwrap } from "@/lib/http";

describe("unwrap", () => {
  it("returns data for an ok envelope", () => {
    expect(unwrap({ status: "ok", data: { id: 1 } })).toEqual({ id: 1 });
  });

  it("throws on an error envelope", () => {
    expect(() =>
      unwrap({ status: "error", error: { message: "nope" } }),
    ).toThrow();
  });
});
