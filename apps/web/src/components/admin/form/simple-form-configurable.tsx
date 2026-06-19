import {
  Children,
  useEffect,
  isValidElement,
  type ReactElement,
  type ReactNode,
} from "react";
import {
  usePreference,
  useResourceContext,
  useStore,
  useTranslate,
  useSetInspectorTitle,
} from "shadmin-core";

import { Configurable } from "@/components/admin/inspector/configurable";
import {
  FieldsSelector,
  type SelectableField,
} from "@/components/admin/inspector/fields-selector";
import {
  SimpleForm,
  type SimpleFormProps,
} from "@/components/admin/form/simple-form";

const EMPTY_ARRAY: SelectableField[] = [];

/**
 * A {@link SimpleForm} whose visible inputs can be reordered or hidden by the
 * end user via the {@link Inspector}.
 *
 * Drop this component anywhere a `SimpleForm` would go. It wraps the form in a
 * {@link Configurable} whose editor is a {@link FieldsSelector}. When the user
 * enters preferences edit mode (see {@link InspectorButton}) and clicks the
 * cogwheel near this form, the Inspector renders the inputs picker.
 *
 * Preferences are persisted under `preferences.${preferenceKey}` (defaulting
 * to `preferences.${resource}.simpleForm`). The user's input order and
 * selection survive page reloads.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/simple-form-configurable SimpleFormConfigurable documentation}
 *
 * @example
 * import { Edit, SimpleFormConfigurable, TextInput } from "@/components/admin";
 *
 * const PostEdit = () => (
 *   <Edit>
 *     <SimpleFormConfigurable omit={["id"]}>
 *       <TextInput source="id" />
 *       <TextInput source="title" />
 *       <TextInput source="body" />
 *     </SimpleFormConfigurable>
 *   </Edit>
 * );
 */
function SimpleFormConfigurable({
  preferenceKey,
  omit,
  ...props
}: SimpleFormConfigurableProps) {
  const translate = useTranslate();
  const resource = useResourceContext();
  const finalPreferenceKey = preferenceKey || `${resource}.simpleForm`;

  const [availableInputs = EMPTY_ARRAY, setAvailableInputs] = useStore<
    SelectableField[]
  >(`preferences.${finalPreferenceKey}.availableInputs`, EMPTY_ARRAY);

  const [, setOmit] = useStore<string[]>(
    `preferences.${finalPreferenceKey}.omit`,
    omit,
  );

  // biome-ignore lint/correctness/useExhaustiveDependencies: this effect re-initializes the persisted input list only when availableInputs is empty (first render or preferences cleared). It intentionally omits children/translate/omit and the store setters so a later child or translation change cannot clobber the user's saved input order and visibility.
  useEffect(() => {
    // first render, or the preferences have been cleared
    const inputs: SelectableField[] = [];
    Children.forEach(props.children, (child, index) => {
      if (!isValidElement<{ source?: string; label?: ReactNode }>(child)) {
        return;
      }
      inputs.push({
        index: String(index),
        source: child.props.source ?? `input-${index}`,
        label:
          child.props.source || child.props.label
            ? child.props.label
            : translate("ra.configurable.SimpleForm.unlabeled", {
                input: index,
                _: `Unlabeled input #%{input}`,
              }),
      });
    });

    if (inputs.length !== availableInputs.length) {
      setAvailableInputs(inputs);
      setOmit(omit || []);
    }
    // Re-initialize only when the persisted availableInputs list is
    // cleared. Re-running on every child / translate change would clobber
    // the user's saved order.
  }, [availableInputs]);

  return (
    <Configurable
      editor={<SimpleFormEditor />}
      preferenceKey={finalPreferenceKey}
      className="block"
    >
      <SimpleFormWithPreferences {...props} />
    </Configurable>
  );
}

/**
 * Filters the children of {@link SimpleForm} based on the user's saved
 * preferences for input order and visibility.
 *
 * @internal
 */
const SimpleFormWithPreferences = ({ children, ...props }: SimpleFormProps) => {
  const [availableInputs = []] = usePreference<SelectableField[]>(
    "availableInputs",
    [],
  );
  const [omit] = usePreference<string[]>("omit", []);
  const [inputs] = usePreference<string[]>(
    "inputs",
    availableInputs
      .filter((input) => !omit?.includes(input.source))
      .map((input) => input.index),
  );
  const childrenArray = Children.toArray(children) as ReactElement[];
  return (
    <SimpleForm {...props}>
      {inputs === undefined
        ? children
        : inputs
            .map((index) => childrenArray[Number(index)])
            .filter((child): child is ReactElement => child != null)}
    </SimpleForm>
  );
};

/**
 * Inspector editor for {@link SimpleFormConfigurable}. Sets the inspector
 * title and shows a {@link FieldsSelector} backed by the form-specific
 * preference keys.
 *
 * @internal
 */
const SimpleFormEditor = () => {
  useSetInspectorTitle("ra.configurable.SimpleForm.title", { _: "Form" });
  return <FieldsSelector name="inputs" availableName="availableInputs" />;
};

interface SimpleFormConfigurableProps extends SimpleFormProps {
  /**
   * Key used to store user preferences for this form. Defaults to
   * `${resource}.simpleForm`. Pass a custom key when several
   * `<SimpleFormConfigurable>` instances render in the same resource.
   */
  preferenceKey?: string;
  /**
   * Inputs to hide by default. Users can re-enable them from the Inspector.
   *
   * @example
   * // hide id and author by default
   * <SimpleFormConfigurable omit={["id", "author"]}>...</SimpleFormConfigurable>
   */
  omit?: string[];
}

export { SimpleFormConfigurable, type SimpleFormConfigurableProps };
