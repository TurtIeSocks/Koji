"use client";
import { Eraser } from "lucide-react";
import {
  OsmFeatureOperator,
  type OsmFeatureOperatorProps,
} from "./osm-feature-operator";

type OsmFeatureSubtractProps = Omit<OsmFeatureOperatorProps, "mode" | "icon">;

const OsmFeatureSubtract = (props: OsmFeatureSubtractProps) => (
  <OsmFeatureOperator
    {...props}
    mode="subtract"
    icon={Eraser}
    label={props.label ?? "Subtract OSM features"}
  />
);

export { type OsmFeatureSubtractProps, OsmFeatureSubtract };
