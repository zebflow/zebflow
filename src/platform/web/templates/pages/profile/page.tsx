import { useState } from "zeb";
import ChromeHeader from "@/pages/home/components/chrome-header";
import Button from "@/components/ui/button";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import CardDescription from "@/components/ui/card-description";
import CardTitle from "@/components/ui/card-title";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";

export const page = {
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-zinc-50 text-gray-900 font-sans",
  },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "Profile",
      description: input?.seo?.description ?? "User profile",
    },
  };
}

export default function Page(input) {
  const user = input?.user ?? {};
  const initialIdentity = input?.effective_git_identity ?? {};
  const [gitName, setGitName] = useState(user?.git_name ?? "");
  const [gitEmail, setGitEmail] = useState(user?.git_email ?? "");
  const [identity, setIdentity] = useState(initialIdentity);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [tone, setTone] = useState("muted");
  const api = input?.profile_api ?? "/api/profile";

  const save = async (event) => {
    event.preventDefault();
    setBusy(true);
    setMessage("");
    setTone("muted");
    try {
      const response = await fetch(api, {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
        },
        body: JSON.stringify({
          git_name: gitName,
          git_email: gitEmail,
        }),
      });
      if (response.status === 401) {
        window.location.href = "/login";
        return;
      }
      const data = await response.json().catch(() => ({}));
      if (!response.ok || data?.ok === false) {
        const err = data?.error?.message ?? data?.error ?? "Failed to save profile";
        setMessage(String(err));
        setTone("error");
        return;
      }
      setIdentity(data?.effective_git_identity ?? {});
      setMessage("Profile saved.");
      setTone("ok");
    } catch (_) {
      setMessage("Failed to save profile.");
      setTone("error");
    } finally {
      setBusy(false);
    }
  };

  const messageClass =
    tone === "ok" ? "text-emerald-700" : tone === "error" ? "text-red-600" : "text-gray-500";
  const source = identity?.source === "user_profile" ? "User profile" : "Generated fallback";

  return (
    <>
      <ChromeHeader />
      <main className="pb-16 pt-24">
        <section className="mx-auto max-w-3xl px-6">
          <header className="mb-8 border-b border-gray-200 pb-4">
            <h1 className="text-3xl font-black tracking-tighter text-gray-900">Profile</h1>
            <p className="mt-2 text-sm text-gray-500">
              Signed in as <span className="font-medium text-gray-700">{input?.owner}</span>.
            </p>
            {input?.app_version ? (
              <p className="mt-1 text-[0.7rem] tracking-wide text-gray-400">v{input.app_version}</p>
            ) : null}
          </header>

          <div className="grid gap-5 md:grid-cols-[1.15fr_0.85fr]">
            <Card>
              <CardContent className="py-5">
                <CardTitle className="text-lg">Git identity</CardTitle>
                <CardDescription className="mt-1">
                  Used for commits, sync rebase, and project history authored from this login.
                </CardDescription>
                <form className="mt-5 flex flex-col gap-4" onSubmit={save}>
                  <Field label="Committer name" id="profile-git-name">
                    <Input
                      id="profile-git-name"
                      value={gitName}
                      onInput={(event) => setGitName(event.currentTarget.value)}
                      placeholder="Your Name"
                      autoComplete="name"
                    />
                  </Field>
                  <Field label="Committer email" id="profile-git-email">
                    <Input
                      id="profile-git-email"
                      type="email"
                      value={gitEmail}
                      onInput={(event) => setGitEmail(event.currentTarget.value)}
                      placeholder="you@example.com"
                      autoComplete="email"
                    />
                  </Field>
                  <div className="flex flex-wrap items-center gap-3 pt-1">
                    <Button type="submit" variant="primary" disabled={busy}>
                      {busy ? "Saving..." : "Save profile"}
                    </Button>
                    <Button as="a" href="/home" variant="outline">
                      Back to home
                    </Button>
                    {message ? <span className={`text-sm ${messageClass}`}>{message}</span> : null}
                  </div>
                </form>
              </CardContent>
            </Card>

            <Card>
              <CardContent className="py-5">
                <CardTitle className="text-lg">Effective identity</CardTitle>
                <div className="mt-4 space-y-3 text-sm">
                  <div>
                    <div className="text-xs font-medium uppercase tracking-wide text-gray-400">Name</div>
                    <div className="mt-1 break-words font-medium text-gray-900">{identity?.name ?? ""}</div>
                  </div>
                  <div>
                    <div className="text-xs font-medium uppercase tracking-wide text-gray-400">Email</div>
                    <div className="mt-1 break-words font-medium text-gray-900">{identity?.email ?? ""}</div>
                  </div>
                  <div>
                    <div className="text-xs font-medium uppercase tracking-wide text-gray-400">Source</div>
                    <div className="mt-1 font-medium text-gray-900">{source}</div>
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>
        </section>
      </main>
    </>
  );
}
