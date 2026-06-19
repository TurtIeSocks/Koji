import type { OsmTagFilter } from "./osm-presets";

/**
 * Curated typed unions of well-known OSM tags in `key=value` form, partitioned
 * by OSM top-level key (one sub-union per category). Source:
 * https://wiki.openstreetmap.org/wiki/Map_features
 *
 * Use `*` as the value for "any value of this key" (e.g. `"building=*"`).
 *
 * The unions are intentionally not exhaustive — OSM defines thousands of tags.
 * Use the {@link OsmTagInput} escape hatch when the union is outdated:
 *   const tags: OsmTagInput[] = ["natural=water", "custom_key=foo"];
 */

// ──────────────────────────────────────────────────────────────────────────
// natural
// ──────────────────────────────────────────────────────────────────────────
type NaturalTag =
  | "natural=water"
  | "natural=wood"
  | "natural=forest"
  | "natural=peak"
  | "natural=ridge"
  | "natural=cliff"
  | "natural=tree"
  | "natural=tree_row"
  | "natural=scrub"
  | "natural=heath"
  | "natural=wetland"
  | "natural=glacier"
  | "natural=bay"
  | "natural=strait"
  | "natural=beach"
  | "natural=coastline"
  | "natural=sand"
  | "natural=dune"
  | "natural=mud"
  | "natural=grassland"
  | "natural=fell"
  | "natural=bare_rock"
  | "natural=scree"
  | "natural=shingle"
  | "natural=cave_entrance"
  | "natural=spring"
  | "natural=hot_spring"
  | "natural=geyser"
  | "natural=volcano"
  | "natural=valley"
  | "natural=hill"
  | "natural=saddle"
  | "natural=sinkhole"
  | "natural=arch"
  | "natural=rock"
  | "natural=stone"
  | "natural=reef"
  | "natural=cape"
  | "natural=isthmus";

// ──────────────────────────────────────────────────────────────────────────
// building
// ──────────────────────────────────────────────────────────────────────────
type BuildingTag =
  | "building=*"
  | "building=yes"
  | "building=apartments"
  | "building=residential"
  | "building=house"
  | "building=detached"
  | "building=semidetached_house"
  | "building=terrace"
  | "building=bungalow"
  | "building=dormitory"
  | "building=cabin"
  | "building=static_caravan"
  | "building=hotel"
  | "building=commercial"
  | "building=industrial"
  | "building=office"
  | "building=retail"
  | "building=supermarket"
  | "building=warehouse"
  | "building=kiosk"
  | "building=service"
  | "building=garage"
  | "building=garages"
  | "building=carport"
  | "building=parking"
  | "building=shed"
  | "building=church"
  | "building=mosque"
  | "building=temple"
  | "building=synagogue"
  | "building=cathedral"
  | "building=chapel"
  | "building=monastery"
  | "building=shrine"
  | "building=school"
  | "building=university"
  | "building=college"
  | "building=kindergarten"
  | "building=public"
  | "building=hospital"
  | "building=fire_station"
  | "building=police"
  | "building=barracks"
  | "building=bunker"
  | "building=stadium"
  | "building=hangar"
  | "building=greenhouse"
  | "building=barn"
  | "building=farm"
  | "building=farm_auxiliary"
  | "building=stable"
  | "building=transformer_tower"
  | "building=water_tower";

// ──────────────────────────────────────────────────────────────────────────
// landuse
// ──────────────────────────────────────────────────────────────────────────
type LandUseTag =
  | "landuse=residential"
  | "landuse=commercial"
  | "landuse=retail"
  | "landuse=industrial"
  | "landuse=military"
  | "landuse=farmland"
  | "landuse=farmyard"
  | "landuse=meadow"
  | "landuse=orchard"
  | "landuse=vineyard"
  | "landuse=forest"
  | "landuse=cemetery"
  | "landuse=quarry"
  | "landuse=brownfield"
  | "landuse=greenfield"
  | "landuse=construction"
  | "landuse=landfill"
  | "landuse=recreation_ground"
  | "landuse=village_green"
  | "landuse=allotments"
  | "landuse=grass"
  | "landuse=plant_nursery"
  | "landuse=aquaculture"
  | "landuse=salt_pond"
  | "landuse=port"
  | "landuse=basin"
  | "landuse=reservoir"
  | "landuse=railway";

// ──────────────────────────────────────────────────────────────────────────
// amenity
// ──────────────────────────────────────────────────────────────────────────
type AmenityTag =
  | "amenity=bar"
  | "amenity=restaurant"
  | "amenity=cafe"
  | "amenity=fast_food"
  | "amenity=pub"
  | "amenity=biergarten"
  | "amenity=food_court"
  | "amenity=ice_cream"
  | "amenity=nightclub"
  | "amenity=atm"
  | "amenity=bank"
  | "amenity=bureau_de_change"
  | "amenity=school"
  | "amenity=university"
  | "amenity=college"
  | "amenity=kindergarten"
  | "amenity=library"
  | "amenity=research_institute"
  | "amenity=hospital"
  | "amenity=clinic"
  | "amenity=doctors"
  | "amenity=dentist"
  | "amenity=pharmacy"
  | "amenity=veterinary"
  | "amenity=nursing_home"
  | "amenity=social_facility"
  | "amenity=childcare"
  | "amenity=bench"
  | "amenity=bicycle_parking"
  | "amenity=bicycle_rental"
  | "amenity=bicycle_repair_station"
  | "amenity=boat_rental"
  | "amenity=car_rental"
  | "amenity=car_sharing"
  | "amenity=car_wash"
  | "amenity=charging_station"
  | "amenity=fuel"
  | "amenity=ferry_terminal"
  | "amenity=parking"
  | "amenity=parking_entrance"
  | "amenity=parking_space"
  | "amenity=taxi"
  | "amenity=place_of_worship"
  | "amenity=grave_yard"
  | "amenity=community_centre"
  | "amenity=conference_centre"
  | "amenity=exhibition_centre"
  | "amenity=events_venue"
  | "amenity=music_venue"
  | "amenity=arts_centre"
  | "amenity=cinema"
  | "amenity=theatre"
  | "amenity=fountain"
  | "amenity=stage"
  | "amenity=marketplace"
  | "amenity=toilets"
  | "amenity=shower"
  | "amenity=crematorium"
  | "amenity=courthouse"
  | "amenity=embassy"
  | "amenity=post_office"
  | "amenity=post_box"
  | "amenity=prison"
  | "amenity=police"
  | "amenity=fire_station"
  | "amenity=ranger_station"
  | "amenity=drinking_water"
  | "amenity=bbq"
  | "amenity=shelter"
  | "amenity=waste_basket"
  | "amenity=waste_disposal"
  | "amenity=recycling";

// ──────────────────────────────────────────────────────────────────────────
// leisure
// ──────────────────────────────────────────────────────────────────────────
type LeisureTag =
  | "leisure=park"
  | "leisure=playground"
  | "leisure=pitch"
  | "leisure=sports_centre"
  | "leisure=stadium"
  | "leisure=swimming_pool"
  | "leisure=water_park"
  | "leisure=track"
  | "leisure=marina"
  | "leisure=nature_reserve"
  | "leisure=garden"
  | "leisure=golf_course"
  | "leisure=miniature_golf"
  | "leisure=horse_riding"
  | "leisure=fitness_centre"
  | "leisure=dance"
  | "leisure=escape_game"
  | "leisure=ice_rink"
  | "leisure=common"
  | "leisure=resort";

// ──────────────────────────────────────────────────────────────────────────
// highway
// ──────────────────────────────────────────────────────────────────────────
type HighwayTag =
  | "highway=motorway"
  | "highway=trunk"
  | "highway=primary"
  | "highway=secondary"
  | "highway=tertiary"
  | "highway=unclassified"
  | "highway=residential"
  | "highway=service"
  | "highway=motorway_link"
  | "highway=trunk_link"
  | "highway=primary_link"
  | "highway=secondary_link"
  | "highway=tertiary_link"
  | "highway=living_street"
  | "highway=pedestrian"
  | "highway=track"
  | "highway=footway"
  | "highway=cycleway"
  | "highway=path"
  | "highway=bridleway"
  | "highway=steps"
  | "highway=construction";

// ──────────────────────────────────────────────────────────────────────────
// waterway
// ──────────────────────────────────────────────────────────────────────────
type WaterwayTag =
  | "waterway=river"
  | "waterway=stream"
  | "waterway=tidal_channel"
  | "waterway=canal"
  | "waterway=drain"
  | "waterway=ditch"
  | "waterway=dock"
  | "waterway=fairway"
  | "waterway=riverbank"
  | "waterway=weir"
  | "waterway=dam"
  | "waterway=waterfall"
  | "waterway=lock_gate";

// ──────────────────────────────────────────────────────────────────────────
// railway
// ──────────────────────────────────────────────────────────────────────────
type RailwayTag =
  | "railway=rail"
  | "railway=subway"
  | "railway=tram"
  | "railway=light_rail"
  | "railway=funicular"
  | "railway=monorail"
  | "railway=narrow_gauge"
  | "railway=preserved"
  | "railway=miniature"
  | "railway=disused"
  | "railway=abandoned"
  | "railway=station"
  | "railway=halt"
  | "railway=tram_stop"
  | "railway=subway_entrance";

// ──────────────────────────────────────────────────────────────────────────
// boundary
// ──────────────────────────────────────────────────────────────────────────
type BoundaryTag =
  | "boundary=administrative"
  | "boundary=protected_area"
  | "boundary=national_park"
  | "boundary=political"
  | "boundary=maritime"
  | "boundary=border_zone"
  | "boundary=forest"
  | "boundary=historic"
  | "boundary=religious"
  | "boundary=aboriginal_lands";

// ──────────────────────────────────────────────────────────────────────────
// place
// ──────────────────────────────────────────────────────────────────────────
type PlaceTag =
  | "place=continent"
  | "place=country"
  | "place=state"
  | "place=region"
  | "place=province"
  | "place=district"
  | "place=county"
  | "place=municipality"
  | "place=city"
  | "place=town"
  | "place=village"
  | "place=hamlet"
  | "place=borough"
  | "place=suburb"
  | "place=neighbourhood"
  | "place=quarter"
  | "place=farm"
  | "place=allotments"
  | "place=island"
  | "place=islet"
  | "place=ocean"
  | "place=sea"
  | "place=archipelago";

// ──────────────────────────────────────────────────────────────────────────
// man_made (commonly-mapped subset)
// ──────────────────────────────────────────────────────────────────────────
type ManMadeTag =
  | "man_made=bridge"
  | "man_made=chimney"
  | "man_made=communications_tower"
  | "man_made=crane"
  | "man_made=flagpole"
  | "man_made=lighthouse"
  | "man_made=mast"
  | "man_made=mineshaft"
  | "man_made=obelisk"
  | "man_made=observatory"
  | "man_made=offshore_platform"
  | "man_made=pier"
  | "man_made=pipeline"
  | "man_made=pumping_station"
  | "man_made=silo"
  | "man_made=storage_tank"
  | "man_made=surveillance"
  | "man_made=tower"
  | "man_made=utility_pole"
  | "man_made=wastewater_plant"
  | "man_made=watermill"
  | "man_made=water_tower"
  | "man_made=water_well"
  | "man_made=water_works"
  | "man_made=windmill"
  | "man_made=works";

// ──────────────────────────────────────────────────────────────────────────
// shop (top-level subset)
// ──────────────────────────────────────────────────────────────────────────
type ShopTag =
  | "shop=supermarket"
  | "shop=convenience"
  | "shop=bakery"
  | "shop=butcher"
  | "shop=confectionery"
  | "shop=deli"
  | "shop=greengrocer"
  | "shop=alcohol"
  | "shop=wine"
  | "shop=clothes"
  | "shop=shoes"
  | "shop=jewelry"
  | "shop=hairdresser"
  | "shop=optician"
  | "shop=florist"
  | "shop=furniture"
  | "shop=hardware"
  | "shop=mobile_phone"
  | "shop=electronics"
  | "shop=computer"
  | "shop=books"
  | "shop=stationery"
  | "shop=toys"
  | "shop=bicycle"
  | "shop=car"
  | "shop=car_parts"
  | "shop=motorcycle"
  | "shop=garden_centre"
  | "shop=pet"
  | "shop=art"
  | "shop=mall"
  | "shop=department_store";

// ──────────────────────────────────────────────────────────────────────────
// tourism
// ──────────────────────────────────────────────────────────────────────────
type TourismTag =
  | "tourism=hotel"
  | "tourism=motel"
  | "tourism=guest_house"
  | "tourism=hostel"
  | "tourism=chalet"
  | "tourism=apartment"
  | "tourism=camp_pitch"
  | "tourism=caravan_site"
  | "tourism=camp_site"
  | "tourism=alpine_hut"
  | "tourism=wilderness_hut"
  | "tourism=viewpoint"
  | "tourism=attraction"
  | "tourism=artwork"
  | "tourism=museum"
  | "tourism=gallery"
  | "tourism=information"
  | "tourism=picnic_site"
  | "tourism=theme_park"
  | "tourism=zoo"
  | "tourism=aquarium";

// ──────────────────────────────────────────────────────────────────────────
// barrier
// ──────────────────────────────────────────────────────────────────────────
type BarrierTag =
  | "barrier=fence"
  | "barrier=wall"
  | "barrier=hedge"
  | "barrier=retaining_wall"
  | "barrier=city_wall"
  | "barrier=gate"
  | "barrier=lift_gate"
  | "barrier=stile"
  | "barrier=cattle_grid"
  | "barrier=kissing_gate"
  | "barrier=swing_gate"
  | "barrier=turnstile"
  | "barrier=bollard"
  | "barrier=chain"
  | "barrier=handrail"
  | "barrier=cycle_barrier"
  | "barrier=rope"
  | "barrier=log";

// ──────────────────────────────────────────────────────────────────────────
// historic
// ──────────────────────────────────────────────────────────────────────────
type HistoricTag =
  | "historic=monument"
  | "historic=memorial"
  | "historic=castle"
  | "historic=ruins"
  | "historic=archaeological_site"
  | "historic=building"
  | "historic=wreck"
  | "historic=tomb"
  | "historic=wayside_cross"
  | "historic=wayside_shrine"
  | "historic=boundary_stone";

// ──────────────────────────────────────────────────────────────────────────
// power
// ──────────────────────────────────────────────────────────────────────────
type PowerTag =
  | "power=line"
  | "power=minor_line"
  | "power=tower"
  | "power=pole"
  | "power=substation"
  | "power=generator"
  | "power=plant"
  | "power=transformer";

/**
 * Union of all category sub-unions. Preserved as the flat type consumers
 * previously imported — derived from the category types rather than
 * hand-rolled, so the two never drift.
 */
type KnownOsmTag =
  | NaturalTag
  | BuildingTag
  | LandUseTag
  | AmenityTag
  | LeisureTag
  | HighwayTag
  | WaterwayTag
  | RailwayTag
  | BoundaryTag
  | PlaceTag
  | ManMadeTag
  | ShopTag
  | TourismTag
  | BarrierTag
  | HistoricTag
  | PowerTag;

/**
 * Tag input type for `tags` props. Accepts the typed union for autocomplete,
 * but `(string & {})` lets any string through as an escape hatch for tags
 * not yet in {@link KnownOsmTag}.
 *
 * ```ts
 * const t: OsmTagInput[] = ["natural=water", "fictional_key=value"];
 * ```
 */
type OsmTagInput = KnownOsmTag | (string & {});

/**
 * Convert a `"key=value"` or `"key=*"` string into the structured filter shape
 * consumed by Overpass query construction.
 *
 * Throws on malformed input (no `=` separator).
 */
function tagToFilter(tag: string): OsmTagFilter {
  const eq = tag.indexOf("=");
  if (eq < 0) {
    throw new Error(
      `Invalid OSM tag: "${tag}". Expected "key=value" or "key=*".`,
    );
  }
  const key = tag.slice(0, eq);
  const value = tag.slice(eq + 1);
  if (!key) {
    throw new Error(`Invalid OSM tag: "${tag}". Empty key.`);
  }
  if (value === "*") return { key, any: true };
  if (!value) {
    throw new Error(
      `Invalid OSM tag: "${tag}". Empty value (use "${key}=*" for any).`,
    );
  }
  return { key, value };
}

export {
  type NaturalTag,
  type BuildingTag,
  type LandUseTag,
  type AmenityTag,
  type LeisureTag,
  type HighwayTag,
  type WaterwayTag,
  type RailwayTag,
  type BoundaryTag,
  type PlaceTag,
  type ManMadeTag,
  type ShopTag,
  type TourismTag,
  type BarrierTag,
  type HistoricTag,
  type PowerTag,
  type KnownOsmTag,
  type OsmTagInput,
  tagToFilter,
};
