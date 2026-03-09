export const app = {};

export default function Page(input) {
  return (
<header className="px-4 py-2">
  <h1>{input.blog.title}</h1>
  <p>{input.blog.tagline}</p>
</header>
  );
}
