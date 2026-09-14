import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Toggle from "@/components/ui/toggle";
import { Select } from "@/components/ui/select";
import { copyText, routeLabel } from "@/pages/project-studio/settings/components/addressing/addressing-lib";

/**
 * Routes and surfaces — the Cloudflare-shaped part. Hidden under Advanced by
 * default because a fresh project needs none of it: pages at the root and the
 * `_` mounts are the defaults. Shown, it is a table of `host/path → surface`
 * rules plus one switch per surface.
 */
export default function AddressingRoutes({ data, hosts, routes, disabled, onAddRoute, onRemoveRoute, onToggleSurface, busy }) {
  const [open, setOpen] = useState(routes.length > 0 || disabled.length > 0);
  const [host, setHost] = useState(hosts[0] ?? data?.dev_host ?? "");
  const [path, setPath] = useState("/");
  const [surface, setSurface] = useState("files");
  const [copied, setCopied] = useState("");
  const surfaces = Array.isArray(data?.surfaces) ? data.surfaces : [];
  const allHosts = [data?.dev_host, ...hosts].filter(Boolean);

  async function copy(text) {
    if (await copyText(text)) {
      setCopied(text);
      setTimeout(() => setCopied(""), 1200);
    }
  }

  function submit(e) {
    e.preventDefault();
    if (!host) return;
    onAddRoute({ host, path: path.trim() || "/", surface });
    setPath("/");
  }

  return (
    <section>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex items-center gap-2 text-[0.8rem] font-semibold text-foreground"
      >
        <span>{open ? "▾" : "▸"}</span>
        <span>Advanced — routes and surfaces</span>
        {!open ? <span className="font-normal text-muted-foreground">({routes.length} routes, {disabled.length} surfaces off)</span> : null}
      </button>

      {open ? (
        <div className="mt-3 flex flex-col gap-5">
          <div>
            <p className="text-[0.78rem] leading-[1.45] text-muted-foreground">
              What each host serves. Without a route a host serves the whole project: pages at <code>/</code> and the
              other surfaces under <code>/_files</code>, <code>/_ws</code>, <code>/_static</code>, <code>/_ms</code>, <code>/_fs</code>, <code>/_mcp</code>.
              A route mounts one surface at a host and path of your choosing; a host with any route serves only its routes.
              Pages can only be a root.
            </p>
            {routes.length > 0 ? (
              <ul className="mt-3 flex flex-col divide-y divide-border border border-border">
                {routes.map((route, index) => {
                  const label = routeLabel(route);
                  const title = surfaces.find((s) => s.key === route.surface)?.title ?? route.surface;
                  return (
                    <li key={`${label}-${index}`} className="flex flex-wrap items-center gap-3 px-3 py-2">
                      <code className="font-mono text-[0.8rem] text-foreground">{label}</code>
                      <span className="text-[0.76rem] text-muted-foreground">→ {title}</span>
                      <span className="ml-auto flex items-center gap-1">
                        <a href={`https://${label}`} target="_blank" rel="noreferrer" className="px-2 text-[0.78rem] text-info underline-offset-4 hover:underline">Open ↗</a>
                        <Button type="button" size="sm" variant="ghost" onClick={() => copy(`https://${label}`)}>{copied === `https://${label}` ? "Copied" : "Copy"}</Button>
                        <Button type="button" size="sm" variant="ghost" onClick={() => onRemoveRoute(index)} disabled={busy}>Remove</Button>
                      </span>
                    </li>
                  );
                })}
              </ul>
            ) : null}
            <form onSubmit={submit} className="mt-3 flex flex-wrap items-center gap-2">
              <Select value={host} onChange={(e) => setHost(e.target.value)} className="w-[16rem] font-mono">
                {allHosts.map((name) => (
                  <option key={name} value={name}>{name}</option>
                ))}
              </Select>
              <Input id="addressing-route-path" value={path} onChange={(e) => setPath(e.target.value)} placeholder="/service/" className="w-[10rem] font-mono" />
              <Select value={surface} onChange={(e) => setSurface(e.target.value)} className="w-[14rem]">
                {surfaces.map((s) => (
                  <option key={s.key} value={s.key}>{s.title}</option>
                ))}
              </Select>
              <Button type="submit" size="sm" variant="outline" disabled={busy || !host}>Add route</Button>
            </form>
          </div>

          <div>
            <h5 className="text-[0.78rem] font-semibold text-foreground">Surfaces</h5>
            <p className="mt-1 text-[0.78rem] text-muted-foreground">Off means 404 on every host, including <code>{data?.neutral_url}</code>.</p>
            <ul className="mt-2 flex flex-col divide-y divide-border border border-border">
              {surfaces.map((s) => (
                <li key={s.key} className="flex flex-wrap items-center gap-3 px-3 py-2">
                  <Toggle
                    label={s.title}
                    checked={!disabled.includes(s.key)}
                    onChange={() => onToggleSurface(s.key)}
                    disabled={busy || s.key === "pages"}
                  />
                  <code className="font-mono text-[0.74rem] text-muted-foreground">{s.default_path}</code>
                  <span className="text-[0.74rem] text-muted-foreground">·</span>
                  <code className="font-mono text-[0.74rem] text-muted-foreground">{s.platform_path}</code>
                  <Button type="button" size="sm" variant="ghost" className="ml-auto" onClick={() => copy(s.platform_path)}>{copied === s.platform_path ? "Copied" : "Copy"}</Button>
                </li>
              ))}
            </ul>
          </div>
        </div>
      ) : null}
    </section>
  );
}
