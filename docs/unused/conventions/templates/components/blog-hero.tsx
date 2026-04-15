export const app = {};

export default function Page(input) {
  return (
<section className="px-4 py-2">
  <h2>{input.hero.title}</h2>
  <p>{input.hero.subtitle}</p>
</section>
  );
}
