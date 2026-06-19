"use client";

import type { ReactElement, ReactNode } from "react";
import { Children, isValidElement, useState } from "react";
import type { RaRecord } from "shadmin-core";
import {
  OptionalRecordContextProvider,
  useLocation,
  useParams,
  useRecordContext,
  useRouterProvider,
  useSplatPathBase,
  useTranslate,
} from "shadmin-core";
import { useNavigate } from "react-router";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { Labeled } from "@/components/admin/views/labeled";

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Returns the URL path segment for a tab.
 * First tab = '' (no segment), subsequent = their index or a custom `path`.
 */
function getShowTabPath(
  tab: ReactElement<ShowTabProps>,
  index: number,
): string {
  return tab.props.path != null
    ? tab.props.path
    : index > 0
      ? index.toString()
      : "";
}

// ---------------------------------------------------------------------------
// ShowTab
// ---------------------------------------------------------------------------

/**
 * A single tab declaration inside `<TabbedShowLayout>`.
 *
 * Acts as a configuration component — `TabbedShowLayout` reads its props and
 * renders the trigger + content itself. `ShowTab` renders nothing on its own.
 *
 * Available as `TabbedShowLayout.Tab`.
 *
 * @example
 * <TabbedShowLayout>
 *   <TabbedShowLayout.Tab label="Summary">
 *     <TextField source="title" />
 *     <TextField source="author" />
 *   </TabbedShowLayout.Tab>
 *   <TabbedShowLayout.Tab label="Details" path="details">
 *     <NumberField source="views" />
 *   </TabbedShowLayout.Tab>
 * </TabbedShowLayout>
 */
function ShowTab(_props: ShowTabProps): null {
  return null;
}

ShowTab.displayName = "ShowTab";

interface ShowTabProps {
  /** Tab label shown in the trigger. */
  label: string | ReactElement;
  /** Fields/components to display in the tab panel. */
  children?: ReactNode;
  /** Custom URL segment for this tab (syncWithLocation only). */
  path?: string;
  /** Icon displayed before the label. */
  icon?: ReactElement;
  /** Optional count badge shown in the trigger, e.g. `count={27}`. */
  count?: ReactNode;
  /** Additional CSS class applied to the tab panel content wrapper. */
  contentClassName?: string;
}

// ---------------------------------------------------------------------------
// TabbedShowLayout
// ---------------------------------------------------------------------------

/**
 * A tabbed layout for Show views that groups fields into navigable tabs.
 *
 * Pulls the current record from `RecordContext`. Each `<TabbedShowLayout.Tab>`
 * child declares a tab; fields inside are automatically wrapped in `<Labeled>`
 * to display their translated labels.
 *
 * When `syncWithLocation` is `true` (the default), changing tabs updates the
 * URL, enabling deep-linking and browser back/forward navigation.
 *
 * @see {@link https://marmelab.com/react-admin/TabbedShowLayout.html}
 *
 * @example
 * import { Show, TabbedShowLayout, TextField, NumberField } from '@/components/admin';
 *
 * const PostShow = () => (
 *   <Show>
 *     <TabbedShowLayout>
 *       <TabbedShowLayout.Tab label="Content">
 *         <TextField source="title" />
 *         <TextField source="body" />
 *       </TabbedShowLayout.Tab>
 *       <TabbedShowLayout.Tab label="Metadata">
 *         <TextField source="author" />
 *         <NumberField source="views" />
 *       </TabbedShowLayout.Tab>
 *     </TabbedShowLayout>
 *   </Show>
 * );
 */
function TabbedShowLayout(props: TabbedShowLayoutProps) {
  const {
    children,
    className,
    divider,
    record: recordProp,
    syncWithLocation = true,
    tabs,
    spacing,
    value,
  } = props;

  const record = useRecordContext(props);

  if (!record) return null;

  return (
    <OptionalRecordContextProvider value={recordProp}>
      <TabbedShowLayoutView
        className={className}
        divider={divider}
        syncWithLocation={syncWithLocation}
        tabs={tabs}
        spacing={spacing}
        value={value}
      >
        {children}
      </TabbedShowLayoutView>
    </OptionalRecordContextProvider>
  );
}

TabbedShowLayout.Tab = ShowTab;

interface TabbedShowLayoutProps {
  children: ReactNode;
  className?: string;
  /** Optional divider element rendered between each field in a tab. */
  divider?: ReactNode;
  /** Explicit record to display (falls back to RecordContext). */
  record?: RaRecord;
  /** When true (default), tab changes update the URL. */
  syncWithLocation?: boolean;
  /** Custom tab-strip renderer. If provided, replaces the default TabsList. */
  tabs?: ReactElement;
  /** Gap between fields within each tab's content panel (in px). */
  spacing?: number;
  /** Controlled tab value override. When provided, forces the Tabs into controlled mode. */
  value?: string;
}

// ---------------------------------------------------------------------------
// TabbedShowLayoutView (internal rendering engine)
// ---------------------------------------------------------------------------

function TabbedShowLayoutView({
  children,
  className,
  divider,
  syncWithLocation,
  tabs: customTabs,
  spacing,
  value: valueProp,
}: {
  children: ReactNode;
  className?: string;
  divider?: ReactNode;
  syncWithLocation: boolean;
  tabs?: ReactElement;
  spacing?: number;
  value?: string;
}) {
  const { Route, Routes } = useRouterProvider();
  const params = useParams();
  const splatPathBase = useSplatPathBase();
  const location = useLocation();
  const navigate = useNavigate();
  const [localTabIndex, setLocalTabIndex] = useState(0);

  const tabs = (
    Children.toArray(children) as ReactElement<ShowTabProps>[]
  ).filter(isValidElement);

  // Determine the active tab value
  const activeValue =
    valueProp !== undefined
      ? valueProp
      : syncWithLocation
        ? (params["*"] ?? "")
        : localTabIndex.toString();

  const handleValueChange = (value: string) => {
    if (syncWithLocation) {
      const newPathname =
        value === "" ? splatPathBase : `${splatPathBase}/${value}`;
      navigate({ pathname: newPathname, search: location.search });
    } else {
      setLocalTabIndex(parseInt(value, 10));
    }
  };

  const tabsList = customTabs ?? (
    <TabbedShowLayoutTabsList tabs={tabs} syncWithLocation={syncWithLocation} />
  );

  const tabPanels = tabs.map((tab, index) => {
    const tabPath = getShowTabPath(tab, index);
    const value = syncWithLocation ? tabPath : index.toString();
    return (
      <TabsContent key={value} value={value}>
        <div
          className={cn(
            "flex flex-col",
            {
              "gap-4": spacing === undefined,
            },
            tab.props.contentClassName,
          )}
          style={spacing !== undefined ? { gap: spacing * 4 } : undefined}
        >
          {Children.map(tab.props.children, (field) => {
            if (!field || !isValidElement(field)) return null;
            return (
              <>
                <Labeled
                  key={
                    (field as ReactElement<{ source?: string }>).props.source
                  }
                >
                  {field as ReactElement}
                </Labeled>
                {divider}
              </>
            );
          })}
        </div>
      </TabsContent>
    );
  });

  return (
    <div className={cn("tabbed-show-layout", className)}>
      <Tabs
        value={activeValue}
        onValueChange={handleValueChange}
        className="w-full"
      >
        {syncWithLocation ? (
          <Routes>
            <Route path="/*" element={tabsList} />
          </Routes>
        ) : (
          tabsList
        )}
        {tabPanels}
      </Tabs>
    </div>
  );
}

// ---------------------------------------------------------------------------
// TabbedShowLayoutTabsList (public)
// ---------------------------------------------------------------------------

interface TabbedShowLayoutTabsListProps {
  tabs: ReactElement<ShowTabProps>[];
  syncWithLocation: boolean;
}

/**
 * Renders the tab trigger list for a {@link TabbedShowLayout}.
 *
 * Exported so consumers can compose their own custom TabbedShowLayout
 * without forking the entire `TabbedShowLayoutView`.
 */
function TabbedShowLayoutTabsList({
  tabs,
  syncWithLocation,
}: TabbedShowLayoutTabsListProps) {
  return (
    <TabsList className="w-full mb-2">
      {tabs.map((tab, index) => {
        const tabPath = getShowTabPath(tab, index);
        const value = syncWithLocation ? tabPath : index.toString();
        return (
          <ShowTabTrigger
            key={value}
            value={value}
            label={tab.props.label}
            icon={tab.props.icon}
            count={tab.props.count}
          />
        );
      })}
    </TabsList>
  );
}

// ---------------------------------------------------------------------------
// ShowTabTrigger (internal)
// ---------------------------------------------------------------------------

function ShowTabTrigger({
  value,
  label,
  icon,
  count,
}: {
  value: string;
  label: string | ReactElement;
  icon?: ReactElement;
  count?: ReactNode;
}) {
  const translate = useTranslate();

  let tabLabel: ReactNode =
    typeof label === "string" ? translate(label, { _: label }) : label;

  if (count !== undefined) {
    tabLabel = (
      <span>
        {tabLabel} ({count})
      </span>
    );
  }

  return (
    <TabsTrigger value={value}>
      {icon && <span className="mr-1.5">{icon}</span>}
      {tabLabel}
    </TabsTrigger>
  );
}

export {
  TabbedShowLayout,
  type TabbedShowLayoutProps,
  ShowTab,
  type ShowTabProps,
  TabbedShowLayoutTabsList,
  type TabbedShowLayoutTabsListProps,
};
