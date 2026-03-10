import Alert from "@/components/ui/alert";
import Button from "@/components/ui/button";
import Card from "@/components/ui/card";
import CardHeader from "@/components/ui/card-header";
import CardTitle from "@/components/ui/card-title";
import CardDescription from "@/components/ui/card-description";
import CardContent from "@/components/ui/card-content";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";

export const page = {
  head: {
    title: "Login",
    description: "Sign in to Zebflow",
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
  return (
    <>
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

          <Card className="shadow-xl">
            <CardHeader>
              <CardTitle>Sign In</CardTitle>
              <CardDescription>Login to continue to Zebflow platform.</CardDescription>
            </CardHeader>

            <CardContent>
              {input?.error ? (
                <Alert variant="error" className="mb-4">{input.error}</Alert>
              ) : null}

              <form method="post" action="/login" className="space-y-4">
                <Field label="Identifier" id="identifier">
                  <Input
                    type="text"
                    name="identifier"
                    id="identifier"
                    defaultValue={input?.default_identifier ?? ""}
                    required
                    autoComplete="username"
                  />
                </Field>

                <Field label="Password" id="password">
                  <Input
                    type="password"
                    name="password"
                    id="password"
                    required
                    autoComplete="current-password"
                  />
                </Field>

                <Button type="submit" variant="primary" size="lg" className="w-full mt-2">
                  Login
                </Button>
              </form>
            </CardContent>
          </Card>
        </section>
      </main>
    </>
  );
}
