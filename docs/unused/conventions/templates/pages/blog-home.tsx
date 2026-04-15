export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
    links: [
      { rel: "canonical", href: "{{input.seo.canonical}}" }
    ],
    meta: [
      { property: "og:title", content: "{{input.seo.title}}" },
      { property: "og:description", content: "{{input.seo.description}}" }
    ]
  },
  html: {
    lang: "en",
  },
  navigation: "history",
};

export const app = (() => {
return {
    state: {
      ui: {
        filter: ""
      }
    },
    actions: {
      "blog.filter.set": (ctx, payload) => {
        ctx.set("ui.filter", String((payload && payload.value) || ""));
        return "ui.filter";
      }
    }
  };
})();

export default function Page(input) {
  return (
<Page>
    <main>
      <h1>{input.blog.title}</h1>
      <p>{input.blog.tagline}</p>

      <section>
        <article>
          <h2><a href="{input.posts[0].url}">{input.posts[0].title}</a></h2>
          <p>{input.posts[0].excerpt}</p>
        </article>
        <article>
          <h2><a href="{input.posts[1].url}">{input.posts[1].title}</a></h2>
          <p>{input.posts[1].excerpt}</p>
        </article>
        <article>
          <h2><a href="{input.posts[2].url}">{input.posts[2].title}</a></h2>
          <p>{input.posts[2].excerpt}</p>
        </article>
      </section>
    </main>
</Page>
  );
}
