"use client";
import { Plus } from "lucide-react";
import {
  OsmFeatureOperator,
  type OsmFeatureOperatorProps,
} from "./osm-feature-operator";

type OsmFeatureAddProps = Omit<OsmFeatureOperatorProps, "mode" | "icon">;

const OsmFeatureAdd = (props: OsmFeatureAddProps) => (
  <OsmFeatureOperator
    {...props}
    mode="add"
    icon={Plus}
    label={props.label ?? "Add OSM features"}
  />
);

export { type OsmFeatureAddProps, OsmFeatureAdd };
