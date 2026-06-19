import type { ReactNode } from "react";
import type { Any } from "@/lib/any";
import { useEffect, useRef, useState } from "react";
import type { InferredTypeMap } from "shadmin-core";
import {
  EditBase,
  InferredElement,
  useResourceContext,
  useEditContext,
  getElementsFromRecords,
} from "shadmin-core";
import { capitalize, singularize } from "inflection";
import type { EditProps } from "@/components/admin/views/edit";
import { EditView } from "@/components/admin/views/edit";
import { SimpleForm } from "@/components/admin/form/simple-form";
import { TextInput } from "@/components/admin/inputs/text-input";
import { BooleanInput } from "@/components/admin/inputs/boolean-input";
import { ReferenceInput } from "@/components/admin/inputs/reference-input";
import { AutocompleteInput } from "@/components/admin/inputs/autocomplete-input";
import { ReferenceArrayInput } from "@/components/admin/inputs/reference-array-input";

/**
 * An edit page that automatically generates a form from your data.
 *
 * Inspects the record to infer field types and automatically creates appropriate inputs.
 * Useful for rapid prototyping. Logs generated code to console.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/edit#scaffolding-an-edit-page EditGuesser documentation}
 *
 * @example
 * import { Admin, EditGuesser } from '@/components/admin';
 * import { Resource } from 'ra-core';
 * import dataProvider from './dataProvider';
 *
 * const App = () => (
 *   <Admin dataProvider={dataProvider}>
 *     ...
 *     <Resource name="customers" edit={EditGuesser} />
 *   </Admin>
 * );
 */
function EditGuesser(props: EditGuesserProps) {
  const {
    disableAuthentication,
    id,
    mutationMode,
    mutationOptions,
    queryOptions,
    redirect,
    resource,
    transform,
    ...rest
  } = props;
  return (
    <EditBase
      disableAuthentication={disableAuthentication}
      id={id}
      mutationMode={mutationMode}
      mutationOptions={mutationOptions}
      queryOptions={queryOptions}
      redirect={redirect}
      resource={resource}
      transform={transform}
    >
      <EditViewGuesser {...rest} />
    </EditBase>
  );
}

function EditViewGuesser(
  props: Omit<EditGuesserProps, EditBaseControllerProps>,
) {
  const resource = useResourceContext();

  if (!resource) {
    throw new Error(`Cannot use <EditGuesser> outside of a ResourceContext`);
  }

  const { record } = useEditContext();
  const [child, setChild] = useState<ReactNode>(null);
  const hasInferredRef = useRef(false);
  const { enableLog = import.meta.env.DEV, ...rest } = props;

  // biome-ignore lint/correctness/useExhaustiveDependencies: reset effect is intentionally keyed on resource only — it clears the inferred view so the next effect re-derives it when the resource changes.
  useEffect(() => {
    hasInferredRef.current = false;
    setChild(null);
  }, [resource]);

  const hasRecord = !!record;
  // biome-ignore lint/correctness/useExhaustiveDependencies: the guesser derives the default form once from the first record shape (gated by hasInferredRef); intentionally not reactive to later record mutations, which would regenerate the UI and re-spam the console log.
  useEffect(() => {
    if (hasInferredRef.current || !hasRecord || !record) return;
    hasInferredRef.current = true;
    const inferredElements = getElementsFromRecords([record], editFieldTypes);
    const inferredChild = new InferredElement(
      editFieldTypes.form,
      null,
      inferredElements,
    );
    setChild(inferredChild.getElement());

    if (!enableLog) return;
    const representation = inferredChild.getRepresentation();
    const components = ["Edit"]
      .concat(
        Array.from(
          new Set(
            Array.from(representation.matchAll(/<([^/\s>]+)/g))
              .map((match) => match[1])
              .filter((component) => component !== "span"),
          ),
        ),
      )
      .sort();
    console.log(
      `Guessed Edit:

${components
  .map(
    (component) =>
      `import { ${component} } from "@/components/admin/${kebabCase(
        component,
      )}";`,
  )
  .join("\n")}

export const ${capitalize(singularize(resource))}Edit = () => (
    <Edit>
${representation}
    </Edit>
);`,
    );
  }, [hasRecord, resource, enableLog]);

  return <EditView {...rest}>{child}</EditView>;
}

type EditBaseControllerProps =
  | "disableAuthentication"
  | "id"
  | "mutationMode"
  | "mutationOptions"
  | "queryOptions"
  | "redirect"
  | "resource"
  | "transform";

interface EditGuesserProps extends Omit<EditProps, "children"> {
  enableLog?: boolean;
}

const editFieldTypes: InferredTypeMap = {
  form: {
    component: (props: Any) => <SimpleForm {...props} />,
    representation: (
      _props: Any,
      children: { getRepresentation: () => string }[],
    ) => `        <SimpleForm>
${children
  .map((child) => `            ${child.getRepresentation()}`)
  .join("\n")}
        </SimpleForm>`,
  },
  reference: {
    component: (props: Any) => (
      <ReferenceInput source={props.source} reference={props.reference}>
        <AutocompleteInput />
      </ReferenceInput>
    ),
    representation: (props: Any) =>
      `<ReferenceInput source="${props.source}" reference="${props.reference}">
                  <AutocompleteInput />
              </ReferenceInput>`,
  },
  referenceArray: {
    component: (props: Any) => <ReferenceArrayInput {...props} />,
    representation: (props: Any) =>
      `<ReferenceArrayInput source="${props.source}" reference="${props.reference}" />`,
  },
  boolean: {
    component: (props: Any) => <BooleanInput {...props} />,
    representation: (props: Any) => `<BooleanInput source="${props.source}" />`,
  },
  string: {
    component: (props: Any) => <TextInput {...props} />,
    representation: (props: Any) => `<TextInput source="${props.source}" />`,
  },
};

const kebabCase = (name: string) => {
  return name
    .replace(/([a-z])([A-Z])/g, "$1-$2")
    .replace(/([A-Z])([A-Z][a-z])/g, "$1-$2")
    .toLowerCase();
};

export { EditGuesser };
