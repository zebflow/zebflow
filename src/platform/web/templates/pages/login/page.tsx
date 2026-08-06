export const page = {
  html: {
    lang: "en",
  },
  body: {
    className:
      "min-h-screen overflow-hidden bg-ui-bg-muted text-ui-text font-sans",
  },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "Login",
      description: input?.seo?.description ?? "Sign in to Zebflow",
    },
  };
}

export default function Page(input) {
  return (
    <main
      className="relative flex min-h-screen w-full items-center justify-center px-5 py-8"
      style={{
        backgroundColor: "var(--color-zeb-bg)",
        fontFamily: "var(--font-sans)",
      }}
    >
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0"
        style={{
          backgroundImage:
            "linear-gradient(var(--color-zeb-grid) 1px, transparent 1px), linear-gradient(90deg, var(--color-zeb-grid) 1px, transparent 1px)",
          backgroundSize: "64px 64px",
          opacity: 0.45,
        }}
      />
      <form
        method="post"
        action="/login"
        className="relative w-full max-w-[340px]"
      >
        <div className="mb-10 flex items-center justify-center gap-3 text-center">
          <img
            src="/assets/branding/logo.svg"
            alt="Zebflow"
            className="h-11 w-11 shrink-0"
          />
          <span className="text-[22px] font-bold leading-none tracking-[-0.01em] text-ui-text">
            zebflow
          </span>
        </div>
        {input?.error ? (
          <p className="mb-3 rounded-[10px] border border-red-200 bg-red-50 px-4 py-3 text-center text-sm font-medium text-red-700">
            {input.error}
          </p>
        ) : null}
        <div>
          <input
            type="text"
            name="identifier"
            placeholder="Username"
            defaultValue={input?.default_identifier ?? ""}
            required
            autoComplete="username"
            className="w-full rounded-[10px] border border-ui-border bg-ui-bg px-4 py-3.5 text-[15px] text-ui-text outline-none transition-colors placeholder:text-ui-text-muted focus:border-brand-blue"
          />
        </div>
        <div className="mt-3">
          <input
            type="password"
            name="password"
            placeholder="Password"
            required
            autoComplete="current-password"
            className="w-full rounded-[10px] border border-ui-border bg-ui-bg px-4 py-3.5 text-[15px] text-ui-text outline-none transition-colors placeholder:text-ui-text-muted focus:border-brand-blue"
          />
        </div>
        <button
          type="submit"
          className="mt-4 block w-full rounded-[10px] bg-dark-background px-4 py-3.5 text-center text-[15.5px] font-semibold text-white transition-colors hover:bg-dark-menus"
        >
          Sign in
        </button>
        {input?.app_version ? (
          <p
            className="mt-7 text-center text-[11px] text-ui-text-muted"
            style={{ fontFamily: "var(--font-mono)" }}
          >
            v{input.app_version}
          </p>
        ) : null}
      </form>
    </main>
  );
}
