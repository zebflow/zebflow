export const page = {
  head: {
    title: "{{input.post.seoTitle}}",
    description: "{{input.post.seoDescription}}",
    links: [
      { rel: "canonical", href: "{{input.post.url}}" }
    ],
    meta: [
      { property: "og:type", content: "article" },
      { property: "og:title", content: "{{input.post.seoTitle}}" },
      { property: "og:description", content: "{{input.post.seoDescription}}" }
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
      postUi: {
        likes: 0
      }
    },
    actions: {
      "post.like": (ctx) => {
        const current = Number(ctx.get("postUi.likes") || 0);
        ctx.set("postUi.likes", current + 1);
        return "postUi.likes";
      }
    },
    memo: {
      "post.likeLabel": (ctx) => `Likes: ${Number(ctx.get("postUi.likes") || 0)}`
    }
  };
})();

export default function Page(input) {
  return (
<Page>
    <article>
      <h1>{input.post.title}</h1>
      <p>By {input.post.author} · {input.post.publishedAt}</p>
      <p>{input.post.summary}</p>
      <div>
        <p>{input.post.body[0]}</p>
        <p>{input.post.body[1]}</p>
        <p>{input.post.body[2]}</p>
      </div>
    </article>
</Page>
  );
}
