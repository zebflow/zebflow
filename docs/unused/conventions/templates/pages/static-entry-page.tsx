export const page = {
  head: {
    title: `${input?.entry?.title ?? "Entry"} - ${input?.collection?.name ?? "Collection"}`,
    description:
      input?.entry?.summary ??
      `Static entry ${input?.entry?.title ?? "entry"} in ${input?.collection?.name ?? "collection"}`,
  },
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-stone-50 text-stone-900 font-serif",
  },
  navigation: "history",
};

function Breadcrumb({ collectionSlug, entrySlug }) {
  return (
    <nav className="text-xs uppercase tracking-[0.2em] text-stone-500">
      <a href="/">Home</a>
      <span className="px-2">/</span>
      <a href={`/collections/${collectionSlug}/index.html`}>Collection</a>
      <span className="px-2">/</span>
      <a href={`/collections/${collectionSlug}/${entrySlug}/index.html`}>Entry</a>
    </nav>
  );
}

export default function Page(input) {
  const collection = input?.collection ?? {};
  const entry = input?.entry ?? {};
  const labels = Array.isArray(input?.related_labels) ? input.related_labels : [];
  const sections = String(entry?.body ?? "")
    .split(/\n{2,}/)
    .map((block) => block.trim())
    .filter(Boolean);

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-4xl flex-col gap-8 px-6 py-10 md:px-10">
      <Breadcrumb collectionSlug={collection.slug} entrySlug={entry.slug} />

      <header className="border-b border-stone-200 pb-6">
        <p className="text-sm font-semibold uppercase tracking-[0.25em] text-amber-700">
          Static Entry Example
        </p>
        <h1 className="mt-3 text-4xl font-black tracking-tight text-stone-950 md:text-5xl">
          {entry.title}
        </h1>
        <p className="mt-3 text-lg text-stone-600">
          In <span className="font-semibold text-stone-900">{collection.name}</span>
        </p>
        {entry.summary ? <p className="mt-4 max-w-2xl text-stone-700">{entry.summary}</p> : null}
      </header>

      <section className="grid gap-8 md:grid-cols-[minmax(0,1fr)_16rem]">
        <article className="rounded-3xl border border-stone-200 bg-white px-6 py-8 shadow-sm">
          <h2 className="text-sm font-bold uppercase tracking-[0.22em] text-stone-500">Content</h2>
          <div className="mt-6 space-y-6 text-lg leading-8 text-stone-900">
            {sections.length > 0 ? (
              sections.map((section, index) => (
                <p key={`section-${index}`} className="whitespace-pre-line">
                  {section}
                </p>
              ))
            ) : (
              <p className="italic text-stone-500">No content yet.</p>
            )}
          </div>
        </article>

        <aside className="space-y-4">
          <div className="rounded-3xl border border-stone-200 bg-stone-100 px-5 py-6">
            <h2 className="text-sm font-bold uppercase tracking-[0.22em] text-stone-500">
              Metadata
            </h2>
            <dl className="mt-4 space-y-3 text-sm">
              <div>
                <dt className="text-stone-500">Collection slug</dt>
                <dd className="font-mono text-stone-900">{collection.slug || "-"}</dd>
              </div>
              <div>
                <dt className="text-stone-500">Entry slug</dt>
                <dd className="font-mono text-stone-900">{entry.slug || "-"}</dd>
              </div>
              <div>
                <dt className="text-stone-500">Generated at</dt>
                <dd className="text-stone-900">{input?.generated_at || "-"}</dd>
              </div>
            </dl>
          </div>

          <div className="rounded-3xl border border-stone-200 bg-white px-5 py-6">
            <h2 className="text-sm font-bold uppercase tracking-[0.22em] text-stone-500">
              Related labels
            </h2>
            <div className="mt-4 flex flex-wrap gap-2">
              {labels.length > 0 ? (
                labels.map((label) => (
                  <span
                    key={label.slug}
                    className="rounded-full bg-amber-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-amber-900"
                  >
                    {label.label}
                  </span>
                ))
              ) : (
                <p className="text-sm text-stone-500">No related labels.</p>
              )}
            </div>
          </div>
        </aside>
      </section>
    </main>
  );
}
