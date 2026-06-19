import { describe, expect, it } from "vitest";
import { dataProvider } from "@/data-provider";

describe("realtime-decorated dataProvider", () => {
  it("retains base CRUD methods", () => {
    expect(dataProvider.getList).toBeTypeOf("function");
    expect(dataProvider.getOne).toBeTypeOf("function");
  });

  it("adds realtime subscribe/publish + lock methods", () => {
    expect(dataProvider.subscribe).toBeTypeOf("function");
    expect(dataProvider.publish).toBeTypeOf("function");
    expect(dataProvider.lock).toBeTypeOf("function");
  });
});
