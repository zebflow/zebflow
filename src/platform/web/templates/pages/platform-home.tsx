import { Link } from "zeb";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import CardTitle from "@/components/ui/card-title";
import CardDescription from "@/components/ui/card-description";

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

export default function Page(input) {
  const projects = Array.isArray(input?.projects) ? input.projects : [];
  return (
    <>
      <nav className="fixed top-0 w-full z-50 bg-white/95 backdrop-blur-sm shadow-sm py-3 border-b border-slate-200">
        <div className="max-w-6xl mx-auto px-6 flex justify-between items-center">
          <div className="flex items-center gap-3 text-xl font-bold tracking-tight text-slate-900">
            <img src="/assets/branding/logo.svg" alt="Zebflow logo" className="w-9 h-9 shrink-0" />
            <span>ZEBFLOW <span className="text-slate-400 ml-2 text-sm">Platform</span></span>
          </div>
          <form method="post" action="/logout">
            <Button type="submit" size="sm" variant="primary" className="rounded-md">
              Logout
            </Button>
          </form>
        </div>
      </nav>

      <main className="pt-24 pb-16">
        <section className="max-w-6xl mx-auto px-6">
          <header className="flex items-end justify-between mb-10 pb-4 border-b border-slate-200">
            <div>
              <h1 className="text-3xl font-black text-slate-900 tracking-tighter">Projects for {input.owner}</h1>
              <p className="text-sm text-slate-500 mt-2">Create and manage your automation projects.</p>
            </div>
          </header>

          <section className="mb-8 bg-white border border-slate-200 rounded-xl p-5">
            <form method="post" action="/home/projects/create" className="grid md:grid-cols-4 gap-3">
              <Input
                type="text"
                name="project"
                placeholder="project slug"
                required
              />
              <Input
                type="text"
                name="title"
                placeholder="project title"
              />
              <Button type="submit" variant="primary" className="font-bold uppercase tracking-widest">
                Create Project
              </Button>

              <Button variant="outline" className="font-bold uppercase tracking-widest">
                See Templates
              </Button>
            </form>
          </section>

          <section className="grid md:grid-cols-2 lg:grid-cols-3 gap-5">
            {projects.map((item, index) => (
              <Link
                key={`${item?.project ?? "project"}-${index}`}
                href={item?.path ?? "#"}
                className="block hover:no-underline"
              >
                <Card className="hover:shadow-md hover:border-slate-300 transition-all cursor-pointer">
                  <CardContent className="py-5">
                    <CardTitle className="text-lg">{item?.title}</CardTitle>
                    <CardDescription className="mt-1">{item?.project}</CardDescription>
                  </CardContent>
                </Card>
              </Link>
            ))}
          </section>
        </section>
      </main>
    </>
  );
}
