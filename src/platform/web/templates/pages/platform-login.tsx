import Button from "@/components/ui/button";

export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
  },
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-zinc-50 text-gray-900 font-sans",
  },
  navigation: "history",
};

export const app = {};

export default function Page(input) {
  return (
    <Page>
        <nav className="fixed top-0 w-full z-50 bg-white/95 backdrop-blur-sm shadow-sm py-3 border-b border-slate-200">
          <div className="max-w-6xl mx-auto px-6 flex justify-between items-center">
            <div className="flex items-center gap-3 text-xl font-bold tracking-tight text-slate-900">
              <img src="/assets/branding/logo.svg" alt="Zebflow logo" className="w-10 h-10 shrink-0" />
              <span>
                ZEBFLOW <span className="text-slate-400 ml-2 text-sm">Platform</span>
              </span>
            </div>
            <div className="text-xs font-mono text-slate-500 uppercase tracking-widest">
              Secure Access
            </div>
          </div>
        </nav>

        <main className="pt-28 pb-14 px-6">
          <section className="max-w-6xl mx-auto grid lg:grid-cols-2 gap-8 items-stretch">
            <article className="bg-gradient-to-br from-slate-900 via-slate-900 to-slate-700 text-white rounded-2xl p-8 border border-slate-800 shadow-2xl">
              <p className="text-xs font-mono uppercase tracking-widest text-slate-300">zebflow / console</p>
              <h1 className="mt-4 text-4xl font-black tracking-tight leading-tight">
                Login To Your
                <br />
                Project Runtime
              </h1>
              <p className="mt-5 text-sm text-slate-200 max-w-md leading-relaxed">
                Continue to pipelines, credentials, tables, and deployment controls with a
                server-first session flow.
              </p>
              <div className="mt-8 space-y-3">
                <p className="text-sm text-slate-100">- Multi-user, multi-project workspace</p>
                <p className="text-sm text-slate-100">- SSR-first pages with TSX authoring</p>
                <p className="text-sm text-slate-100">- Pin-based pipeline orchestration</p>
              </div>
            </article>

            <section className="bg-white border border-slate-200 rounded-2xl shadow-xl overflow-hidden">
              <header className="px-7 py-6 border-b border-slate-100 bg-slate-50">
                <h2 className="text-2xl font-black tracking-tight text-slate-900">Sign In</h2>
                <p className="mt-2 text-sm text-slate-600">
                  Login to continue to Zebflow platform.
                </p>
              </header>

              <div className="px-7 pt-5">
                <div
                  jShow="input.error"
                  className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700"
                >
                  <span jText="input.error">{input.error}</span>
                </div>
              </div>

              <form method="post" action="/login" className="px-7 pt-5 pb-7 space-y-4">
                <div>
                  <label className="block text-xs font-mono uppercase tracking-widest text-slate-500 mb-2">
                    Identifier
                  </label>
                  <input
                    name="identifier"
                    value={input.default_identifier}
                    required
                    autoComplete="username"
                    className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:outline-none focus:border-[#005B9A]"
                  />
                </div>

                <div>
                  <label className="block text-xs font-mono uppercase tracking-widest text-slate-500 mb-2">
                    Password
                  </label>
                  <input
                    type="password"
                    name="password"
                    value={input.default_password}
                    required
                    autoComplete="current-password"
                    className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:outline-none focus:border-[#005B9A]"
                  />
                </div>

                <Button
                  type="submit"
                  label="Login"
                  variant="primary"
                  size="md"
                  className="w-full mt-2"
                />
              </form>
            </section>
          </section>
        </main>
    </Page>
  );
}
