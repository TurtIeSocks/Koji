import type { ReactNode } from "react";
import type { Any } from "@/lib/any";
import { useEffect, useRef, useState, isValidElement, Children } from "react";
import type { InferredTypeMap } from "shadmin-core";
import {
  ShowBase,
  InferredElement,
  getElementsFromRecords,
  useResourceContext,
  useShowContext,
} from "shadmin-core";
import { capitalize, singularize } from "inflection";
import type { ShowProps } from "@/components/admin/views/show";
import { ShowView } from "@/components/admin/views/show";
import { SimpleShowLayout } from "@/components/admin/views/simple-show-layout";
import { RecordField } from "@/components/admin/fields/record-field";
import { DateField } from "@/components/admin/fields/date-field";
import { ReferenceField } from "@/components/admin/fields/reference-field";
import { NumberField } from "@/components/admin/fields/number-field";
import { ArrayField } from "@/components/admin/fields/array-field";
import { BadgeField } from "@/components/admin/fields/badge-field";
import { SingleFieldList } from "@/components/admin/list/single-field-list";
import { ReferenceArrayField } from "@/components/admin/fields/reference-array-field";

/**
 * A show page that automatically generates fields from your data.
 *
 * Inspects the record to infer field types and automatically creates appropriate display fields.
 * Useful for rapid prototyping. Logs generated code to console when enableLog is true.
 *
 * @example
 * import { ShowGuesser } from '@/components/admin';
 *
 * export const PostShow = () => <ShowGuesser enableLog />;
 */
function ShowGuesser(props: ShowGuesserProps) {
  const { disableAuthentication, id, queryOptions, resource, ...rest } = props;
  return (
    <ShowBase
      disableAuthentication={disableAuthentication}
      id={id}
      queryOptions={queryOptions}
      resource={resource}
    >
      <ShowViewGuesser {...rest} />
    </ShowBase>
  );
}

function ShowViewGuesser(
  props: Omit<ShowGuesserProps, ShowBaseControllerProps>,
) {
  const resource = useResourceContext();

  if (!resource) {
    throw new Error(`Cannot use <ShowGuesser> outside of a ResourceContext`);
  }

  const { record } = useShowContext();
  const [child, setChild] = useState<ReactNode>(null);
  const hasInferredRef = useRef(false);
  const { enableLog = import.meta.env.DEV, ...rest } = props;

  // biome-ignore lint/correctness/useExhaustiveDependencies: reset effect is intentionally keyed on resource only — it clears the inferred view so the next effect re-derives it when the resource changes.
  useEffect(() => {
    hasInferredRef.current = false;
    setChild(null);
  }, [resource]);

  const hasRecord = !!record;
  // biome-ignore lint/correctness/useExhaustiveDependencies: the guesser derives the default fields once from the first record shape (gated by hasInferredRef); intentionally not reactive to later record mutations, which would regenerate the UI and re-spam the console log.
  useEffect(() => {
    if (hasInferredRef.current || !hasRecord || !record) return;
    hasInferredRef.current = true;
    const inferredElements = getElementsFromRecords([record], showFieldTypes);
    const inferredChild = new InferredElement(
      showFieldTypes.show,
      null,
      inferredElements,
    );
    setChild(inferredChild.getElement());

    if (!enableLog) return;
    const representation = inferredChild.getRepresentation();
    const components = ["Show"]
      .concat(
        Array.from(
          new Set(
            Array.from(representation.matchAll(/<([^/\s>]+)/g))
              .map((match) => match[1])
              .filter(
                (component) => component !== "span" && component !== "div",
              ),
          ),
        ),
      )
      .sort();
    console.log(
      `Guessed Show:

${components
  .map(
    (component) =>
      `import { ${component} } from "@/components/admin/${kebabCase(
        component,
      )}";`,
  )
  .join("\n")}

export const ${capitalize(singularize(resource))}Show = () => (
    <Show>
${inferredChild.getRepresentation()}
    </Show>
);`,
    );
  }, [hasRecord, resource, enableLog]);

  return <ShowView {...rest}>{child}</ShowView>;
}

type ShowBaseControllerProps =
  | "disableAuthentication"
  | "id"
  | "queryOptions"
  | "resource";

interface ShowGuesserProps extends Omit<ShowProps, "children"> {
  enableLog?: boolean;
}

const showFieldTypes: InferredTypeMap = {
  show: {
    component: (props: Any) => (
      <SimpleShowLayout>{props.children}</SimpleShowLayout>
    ),
    representation: (
      _props: Any,
      children: { getRepresentation: () => string }[],
    ) => `        <SimpleShowLayout>
${children
  .map((child) => `            ${child.getRepresentation()}`)
  .join("\n")}
        </SimpleShowLayout>`,
  },
  reference: {
    component: (props: Any) => (
      <RecordField source={props.source}>
        <ReferenceField source={props.source} reference={props.reference} />
      </RecordField>
    ),
    representation: (props: Any) =>
      `<RecordField source="${props.source}">
                <ReferenceField source="${props.source}" reference="${props.reference}" />
            </RecordField>`,
  },
  array: {
    component: ({ children, ...props }: Any) => {
      const childrenArray = Children.toArray(children);
      return (
        <RecordField source={props.source}>
          <ArrayField source={props.source}>
            <SingleFieldList>
              <BadgeField
                source={
                  childrenArray.length > 0 &&
                  isValidElement(childrenArray[0]) &&
                  (childrenArray[0].props as Any).source
                }
              />
            </SingleFieldList>
          </ArrayField>
        </RecordField>
      );
    },
    representation: (props: Any, children: Any) =>
      `<RecordField source="${props.source}">
                <ArrayField source="${props.source}">
                    <SingleFieldList>
                        <BadgeField source="${
                          children.length > 0 && children[0].getProps().source
                        }" />
                    </SingleFieldList>
                </ArrayField>
            </RecordField>`,
  },
  referenceArray: {
    component: (props: Any) => (
      <RecordField source={props.source}>
        <ReferenceArrayField {...props} />
      </RecordField>
    ),
    representation: (props: Any) =>
      `<RecordField source="${props.source}">
                <ReferenceArrayField source="${props.source}" reference="${props.reference}" />
            </RecordField>`,
  },
  boolean: {
    component: (props: Any) => (
      <RecordField
        source={props.source}
        render={(record) => (record[props.source] ? "Yes" : "No")}
      />
    ),
    representation: (props: Any) =>
      `<RecordField source="${props.source}" render={record => record[${props.source}] ? 'Yes' : 'No'} />`,
  },
  date: {
    component: (props: Any) => (
      <RecordField source={props.source}>
        <DateField source={props.source} />
      </RecordField>
    ),
    representation: (props: Any) =>
      `<RecordField source="${props.source}">
                <DateField source="${props.source}" />
            </RecordField>`,
  },
  number: {
    component: (props: Any) => (
      <RecordField source={props.source}>
        <NumberField source={props.source} />
      </RecordField>
    ),
    representation: (props: Any) =>
      `<RecordField source="${props.source}">
                <NumberField source="${props.source}" />
            </RecordField>`,
  },
  string: {
    component: (props: Any) => <RecordField source={props.source} />,
    representation: (props: Any) => `<RecordField source="${props.source}" />`,
  },
};

const kebabCase = (name: string) => {
  return name
    .replace(/([a-z])([A-Z])/g, "$1-$2")
    .replace(/([A-Z])([A-Z][a-z])/g, "$1-$2")
    .toLowerCase();
};

export { ShowGuesser, type ShowGuesserProps };
