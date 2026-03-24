export const page = {
  head: {
    title: "Login",
    description: "Sign in to Zebflow",
  },
  html: {
    lang: "en",
  },
  body: {
    className:
      "min-h-screen flex flex-col items-center justify-center px-4 py-5 bg-zinc-50 text-slate-900 font-sans",
  },
  navigation: "history",
};

export default function Page(input) {
  return (
    <div className="flex w-full max-w-xs flex-col items-center">
      <div className="flex flex-col items-center gap-2">
        <img
          src="/assets/branding/logo.svg"
          alt="Zebflow"
          className="h-14 w-14 shrink-0"
        />
        <span className="text-xl font-bold tracking-tight text-slate-900">ZEBFLOW</span>
      </div>
      <form
        method="post"
        action="/login"
        className="mt-6 flex w-full flex-col gap-2"
      >
        {input?.error ? (
          <p className="text-center text-sm text-red-600">{input.error}</p>
        ) : null}
        <input
          type="text"
          name="identifier"
          placeholder="Username"
          defaultValue={input?.default_identifier ?? ""}
          required
          autoComplete="username"
          className="h-9 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm outline-none focus:ring-2 focus:ring-slate-400"
        />
        <input
          type="password"
          name="password"
          placeholder="Password"
          required
          autoComplete="current-password"
          className="h-9 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm outline-none focus:ring-2 focus:ring-slate-400"
        />
        <button
          type="submit"
          className="h-10 w-full rounded-lg bg-slate-900 text-sm font-medium text-white hover:opacity-90"
        >
          Sign in
        </button>
      </form>
    </div>
  );
}
