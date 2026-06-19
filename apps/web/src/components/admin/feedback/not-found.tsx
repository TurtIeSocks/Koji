import type { HTMLAttributes } from "react";
import { History, SearchX } from "lucide-react";
import {
  Translate,
  useAuthenticated,
  useDefaultTitle,
  type TitleComponent,
} from "shadmin-core";
import { Button } from "@/components/ui/button";
import { Loading } from "@/components/admin/feedback/loading";
import { Title } from "@/components/admin/layout/title";

type NotFoundProps = Omit<HTMLAttributes<HTMLDivElement>, "title"> & {
  title?: TitleComponent;
};

/**
 * Fallback page displayed when no route matches the current URL.
 */
function NotFound({ title: _title, ...rest }: NotFoundProps) {
  const { isPending } = useAuthenticated();
  const defaultTitle = useDefaultTitle();
  if (isPending) {
    return <Loading />;
  }
  return (
    <div
      className="flex min-h-[50vh] flex-1 flex-col items-center justify-center gap-2 text-center"
      {...rest}
    >
      <Title defaultTitle={defaultTitle} />
      <SearchX className="size-16 text-muted-foreground" />
      <h1 className="text-2xl font-semibold">
        <Translate i18nKey="ra.page.not_found" />
      </h1>
      <p className="max-w-xl text-muted-foreground">
        <Translate i18nKey="ra.message.not_found" />
      </p>
      <Button className="mt-3 cursor-pointer" onClick={goBack}>
        <History />
        <Translate i18nKey="ra.action.back" />
      </Button>
    </div>
  );
}

function goBack() {
  window.history.go(-1);
}

export { NotFound, type NotFoundProps };
