import { describe, expect, it } from "vitest";
import type { WebhookArea } from "@/api/types";
import { groupAreaOptions } from "./dragonite-area-options";

const area = (id: number, name: string, held_by: WebhookArea["held_by"] = []): WebhookArea => ({
  id,
  name,
  held_by,
});

const group = (
  areas: WebhookArea[],
  over: Partial<Parameters<typeof groupAreaOptions>[1]> = {},
) => groupAreaOptions(areas, { geofenceName: "Denver", mode: "pokemon", ...over });

const names = (choices: { name: string }[]) => choices.map((c) => c.name);

describe("groupAreaOptions", () => {
  it("lists every Area A to Z, ignoring case", () => {
    const { rest } = group([area(1, "boulder"), area(2, "Aspen"), area(3, "Vail"), area(4, "avon")]);
    expect(names(rest)).toEqual(["Aspen", "avon", "boulder", "Vail"]);
  });

  it("has no suggestion when no Area carries the geofence's name", () => {
    const { suggested, rest } = group([area(1, "Boulder"), area(2, "Aspen")]);
    expect(suggested).toEqual([]);
    expect(names(rest)).toEqual(["Aspen", "Boulder"]);
  });

  it("suggests the Area with the geofence's name, ignoring case, and does not repeat it", () => {
    const { suggested, rest } = group([area(1, "Boulder"), area(2, "DENVER"), area(3, "Aspen")]);
    expect(names(suggested)).toEqual(["DENVER"]);
    expect(names(rest)).toEqual(["Aspen", "Boulder"]);
  });

  it("names the other geofence of the same mode that holds an Area", () => {
    const { rest } = group([
      area(1, "Boulder", [
        { geofence: 7, name: "Boulder pokemon", mode: "pokemon" },
        { geofence: 8, name: "Boulder quest", mode: "quest" },
      ]),
    ]);
    expect(rest[0].holder).toEqual({ geofence: 7, name: "Boulder pokemon", mode: "pokemon" });
  });

  it("does not count another mode, or the geofence itself, as the holder", () => {
    const held = [
      { geofence: 8, name: "Other mode", mode: "quest" },
      { geofence: 3, name: "Itself", mode: "pokemon" },
    ];
    const { rest } = group([area(1, "Boulder", held)], { selfId: 3 });
    expect(rest[0].holder).toBeNull();
  });
});
