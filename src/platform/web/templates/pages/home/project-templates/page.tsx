import { Link } from "zeb";
import ChromeHeader from "@/pages/home/components/chrome-header";
import Button from "@/components/ui/button";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import CardTitle from "@/components/ui/card-title";
import CardDescription from "@/components/ui/card-description";
import Badge from "@/components/ui/badge";

const BLUEPRINTS = [
  {
    id: "blog-auth",
    title: "Blogging engine with auth",
    blurb:
      "Posts, drafts, comments, and an admin area. Webhook login, JWT cookies, and protected routes wired through pipelines.",
  },
  {
    id: "info-extract",
    title: "Automatic information extraction",
    blurb:
      "Ingest URLs or uploads, normalize text, run summarization and entity tagging. Queue-friendly scripts plus optional LLM nodes.",
  },
  {
    id: "chat-room",
    title: "Chatting room",
    blurb:
      "Rooms and sessions over WebSocket triggers, broadcast events, and shared room state for live messaging prototypes.",
  },
  {
    id: "multiplayer-games",
    title: "Multiplayer web games",
    blurb:
      "Low-latency state patches, session-scoped rooms, and client hydration for canvas or DOM games with server-authoritative ticks.",
  },
  {
    id: "erp",
    title: "Enterprise Resource Planning",
    blurb:
      "Orders, inventory, suppliers, and finance views backed by pipelines and tables. Role-scoped APIs and audit-friendly event flows.",
  },
  {
    id: "bi",
    title: "Business Intelligence",
    blurb:
      "Dashboards fed from warehouse or live queries: charts, KPIs, and scheduled reports. Pair pg.query or Sekejap with web.render for executives.",
  },
  {
    id: "gis",
    title: "Geographic Information System",
    blurb:
      "Layers, features, and spatial filters on a map canvas. zeb/deckgl or threejs-friendly pages with geo pipelines and Tool.geo helpers.",
  },
  {
    id: "fleet-rt",
    title: "Real-time multi vehicles monitoring",
    blurb:
      "Live positions and status over WebSocket rooms, merge-friendly telemetry ingestion, and operator views that stay in sync across clients.",
  },
  {
    id: "avatar-companion",
    title: "Talking companion with avatar",
    blurb:
      "Voice or text chat looped through LLM nodes, lip-sync or VRM avatar in the browser, and session memory scoped to your project policies.",
  },
];

export const page = {
  head: {
    title: ctx?.seo?.title ?? "",
    description: ctx?.seo?.description ?? "",
  },
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-zinc-50 text-gray-900 font-sans",
  },
  navigation: "history",
};

export default function Page() {
  return (
    <>
      <ChromeHeader />

      <main className="pb-16 pt-24">
        <section className="mx-auto max-w-6xl px-6">
          <div className="mb-6">
            <Link
              href="/home"
              className="text-sm font-medium text-slate-600 hover:text-slate-900 hover:underline"
            >
              ← Back to projects
            </Link>
          </div>

          <header className="mb-10 border-b border-slate-200 pb-6">
            <h1 className="text-3xl font-black tracking-tighter text-slate-900">Project templates</h1>
            <p className="mt-2 max-w-2xl text-sm text-slate-500">
              Starter blueprints you can map to new projects. These are illustrative recipes—not one-click installers yet.
            </p>
          </header>

          <ul className="grid gap-5 md:grid-cols-2 lg:grid-cols-3">
            {BLUEPRINTS.map((item) => (
              <li key={item.id}>
                <Card className="h-full border-slate-200 bg-white">
                  <CardContent className="flex h-full flex-col gap-3 py-6">
                    <div className="flex flex-wrap items-start justify-between gap-2">
                      <CardTitle className="text-lg leading-snug">{item.title}</CardTitle>
                      <Badge variant="secondary">Blueprint</Badge>
                    </div>
                    <CardDescription className="text-sm leading-relaxed text-slate-600">
                      {item.blurb}
                    </CardDescription>
                    <div className="mt-auto pt-2">
                      <Button type="button" variant="outline" size="sm" disabled className="cursor-not-allowed opacity-60">
                        Use template (soon)
                      </Button>
                    </div>
                  </CardContent>
                </Card>
              </li>
            ))}
          </ul>
        </section>
      </main>
    </>
  );
}
