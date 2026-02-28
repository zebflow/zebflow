export const page = {
  head: {
    title: "{{input.seo.title}}",
  },
  html: {
    lang: "en",
  },
  navigation: "history",
};

export const app = (() => {
return {
    state: {
      blogUi: {
        mode: "home"
      }
    }
  };
})();

export default function Page(input) {
  return (
<Page>
    <BlogHeader />
    <BlogHero />
    <main>
      <article>
        <h2>{input.posts[0].title}</h2>
      </article>
    </main>
</Page>
  );
}
