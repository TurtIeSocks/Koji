import { useTranslate, useSetInspectorTitle } from "shadmin-core";

/**
 * Default empty-state content for the {@link Inspector}.
 *
 * Sets the inspector title to "Inspector" via {@link useSetInspectorTitle}, and
 * renders a hint message inviting the user to hover the application UI to find
 * a {@link Configurable} element.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/inspector Inspector documentation}
 */
function InspectorRoot() {
  const translate = useTranslate();
  useSetInspectorTitle("ra.configurable.inspector.title", {
    _: "Inspector",
  });

  return (
    <p className="text-sm text-muted-foreground">
      {translate("ra.configurable.inspector.content", {
        _: "Hover the application UI elements to configure them",
      })}
    </p>
  );
}

export { InspectorRoot };
