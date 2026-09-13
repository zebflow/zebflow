import { useState } from "zeb/react";
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
    className: "min-h-screen bg-background text-foreground font-sans",
  },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "Change Password",
      description: input?.seo?.description ?? "Choose a new password",
    },
  };
}

export default function Page(input) {
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [tone, setTone] = useState("muted");
  const api = input?.password_api ?? "/api/profile/password";
  const forced = Boolean(input?.forced);

  const save = async (event) => {
    event.preventDefault();
    if (newPassword !== confirmPassword) {
      setMessage("New password and confirmation do not match.");
      setTone("error");
      return;
    }
    setBusy(true);
    setMessage("");
    setTone("muted");
    try {
      const response = await fetch(api, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
        },
        body: JSON.stringify({
          current_password: currentPassword,
          new_password: newPassword,
        }),
      });
      if (response.status === 401) {
        window.location.href = "/login";
        return;
      }
      const data = await response.json().catch(() => ({}));
      if (!response.ok || data?.ok === false) {
        const err =
          data?.error?.message ?? data?.error ?? "Failed to change the password";
        setMessage(String(err));
        setTone("error");
        return;
      }
      setMessage("Password changed. Redirecting...");
      setTone("ok");
      window.location.href = "/home";
    } catch (_) {
      setMessage("Failed to change the password.");
      setTone("error");
    } finally {
      setBusy(false);
    }
  };

  const messageClass =
    tone === "ok" ? "text-emerald-700" : tone === "error" ? "text-red-600" : "text-gray-500";

  return (
    <main className="flex min-h-screen items-center justify-center px-5 py-10">
      <section className="w-full max-w-md">
        <header className="mb-6 text-center">
          <h1 className="text-2xl font-black tracking-tighter text-gray-900">
            Change password
          </h1>
          <p className="mt-2 text-sm text-gray-500">
            Signed in as{" "}
            <span className="font-medium text-gray-700">{input?.owner}</span>
          </p>
        </header>
        {forced ? (
          <p className="mb-4 rounded-[10px] border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-800">
            This account still uses its generated bootstrap password. Choose a
            new one to continue — every other page redirects here until you do.
          </p>
        ) : null}
        <Card>
          <CardContent className="py-5">
            <CardTitle className="text-lg">New credential</CardTitle>
            <CardDescription className="mt-1">
              Verify the current password, then choose its replacement. Every
              other signed-in session of this account is ended.
            </CardDescription>
            <form className="mt-5 flex flex-col gap-4" onSubmit={save}>
              <Field label="Current password" id="account-current-password">
                <Input
                  id="account-current-password"
                  type="password"
                  value={currentPassword}
                  onInput={(event) => setCurrentPassword(event.currentTarget.value)}
                  autoComplete="current-password"
                  required
                />
              </Field>
              <Field label="New password" id="account-new-password">
                <Input
                  id="account-new-password"
                  type="password"
                  value={newPassword}
                  onInput={(event) => setNewPassword(event.currentTarget.value)}
                  autoComplete="new-password"
                  required
                />
              </Field>
              <Field label="Confirm new password" id="account-confirm-password">
                <Input
                  id="account-confirm-password"
                  type="password"
                  value={confirmPassword}
                  onInput={(event) => setConfirmPassword(event.currentTarget.value)}
                  autoComplete="new-password"
                  required
                />
              </Field>
              <div className="flex flex-wrap items-center gap-3 pt-1">
                <Button type="submit" variant="primary" disabled={busy}>
                  {busy ? "Changing..." : "Change password"}
                </Button>
                {forced ? null : (
                  <Button as="a" href="/profile" variant="outline">
                    Back to profile
                  </Button>
                )}
                {message ? (
                  <span className={`text-sm ${messageClass}`}>{message}</span>
                ) : null}
              </div>
            </form>
          </CardContent>
        </Card>
        {input?.app_version ? (
          <p className="mt-6 text-center text-[11px] text-muted-foreground">
            v{input.app_version}
          </p>
        ) : null}
      </section>
    </main>
  );
}
