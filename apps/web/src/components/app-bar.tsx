import { AppBar, ThemeModeToggle } from "@/components/admin";
import { Map } from "lucide-react";

export function KojiAppBar() {
  return (
    <AppBar>
      <span id="react-admin-title" className="font-semibold" />
      <span className="flex-1" />
      <a
        href="/map"
        title="Open the live map"
        className="inline-flex items-center px-2"
      >
        <Map className="size-4" />
      </a>
      <ThemeModeToggle />
    </AppBar>
  );
}
