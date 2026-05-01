export const page = {
  html: {
    lang: "en",
  },
  body: {
    className:
      "min-h-screen flex flex-col items-center justify-center px-4 py-5 bg-zinc-50 text-gray-900 font-sans",
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
    <div className="flex w-full max-w-xs flex-col items-center">
      <div className="flex flex-col items-center gap-2">
        <img
          src="/assets/branding/logo.svg"
          alt="Zebflow"
          className="h-14 w-14 shrink-0"
        />
        <span className="text-xl font-bold tracking-tight text-gray-900">ZEBFLOW</span>
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
          className="h-9 w-full rounded-lg border border-gray-300 bg-white px-3 text-sm outline-none focus:ring-2 focus:ring-gray-400"
        />
        <input
          type="password"
          name="password"
          placeholder="Password"
          required
          autoComplete="current-password"
          className="h-9 w-full rounded-lg border border-gray-300 bg-white px-3 text-sm outline-none focus:ring-2 focus:ring-gray-400"
        />
        <button
          type="submit"
          className="h-10 w-full rounded-lg bg-gray-900 text-sm font-medium text-white hover:opacity-90"
        >
          Sign in
        </button>
      </form>
      {input?.app_version ? (
        <p className="mt-6 text-[0.7rem] text-gray-400 tracking-wide">v{input.app_version}</p>
      ) : null}
    </div>
  );
}
