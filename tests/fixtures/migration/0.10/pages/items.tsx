export default function Page(input) {
  const items = input.items ?? [];
  return (
    <main>
      <h1 id="t">{input.title}</h1>
      <p id="who">{input.params.who}</p>
      <p id="list">{items.join(",")}</p>
    </main>
  );
}
