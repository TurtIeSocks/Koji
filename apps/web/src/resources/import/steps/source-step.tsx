import { useState } from "react";
import { useFormContext } from "react-hook-form";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { parseSourceText } from "@/lib/geojson-source";
import { postConvert } from "@api";

/** Source step: ingest GeoJSON or lat,lon lists by paste or file, normalize
 *  them server-side via /internal/geometry/convert, and load the resulting
 *  features into the form. Malformed input is surfaced as an error — never
 *  silently dropped. */
function SourceStep({ onLoaded }: { onLoaded: () => void }) {
  const { setValue } = useFormContext();
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [count, setCount] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);

  const load = async () => {
    setError(null);
    const parsed = parseSourceText(text);
    if ("error" in parsed) {
      setError(parsed.error);
      return;
    }
    setLoading(true);
    try {
      const converted = await postConvert(parsed.features);
      setValue("features", converted, { shouldDirty: true });
      // Latch for step 2: once a source loaded, the map stays mounted even if
      // the user deletes every shape (see MapNameStep's empty-state gate).
      setValue("_source_loaded", true);
      setCount(converted.length);
      onLoaded();
    } catch (e) {
      setError(`Convert failed: ${(e as Error).message}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Tabs defaultValue="paste" className="flex flex-col gap-4">
      <TabsList>
        <TabsTrigger value="paste">Paste</TabsTrigger>
        <TabsTrigger value="file">File</TabsTrigger>
      </TabsList>
      <TabsContent value="paste" className="flex flex-col gap-3">
        <label htmlFor="paste-geojson" className="text-sm font-medium">
          Paste GeoJSON or a lat,lon list
        </label>
        <textarea
          id="paste-geojson"
          className="min-h-48 rounded-md border bg-background p-2 font-mono text-sm"
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder='{ "type": "FeatureCollection", ... }  —  or one lat,lon per line'
        />
      </TabsContent>
      <TabsContent value="file" className="flex flex-col gap-3">
        <input
          type="file"
          aria-label="GeoJSON or lat,lon file"
          accept=".json,.geojson,.txt,.csv,application/geo+json,application/json,text/plain,text/csv"
          onChange={async (e) => {
            const file = e.target.files?.[0];
            if (file) setText(await file.text());
          }}
        />
      </TabsContent>
      <div className="flex items-center gap-3">
        <Button onClick={load} disabled={!text.trim() || loading}>
          Load
        </Button>
        {count != null && (
          <span className="text-sm text-muted-foreground">
            {count} feature(s) loaded
          </span>
        )}
      </div>
      {error && (
        <p className="text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
    </Tabs>
  );
}

export { SourceStep };
