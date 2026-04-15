export const page = {
  head: {
    title: "Zebflow Home",
  },
  navigation: "history",
};

export const app = (() => {
return {
    state: {
      home: {
        title: "Zebflow Home",
        description: "Composable pipeline + sandbox + reactive web.",
        visits: 0
      }
    },
    actions: {
      "home.bump": (ctx) => {
        const current = Number(ctx.get("home.visits") || 0);
        ctx.set("home.visits", current + 1);
        return "home.visits";
      }
    }
  };
})();

export default function Page(input) {
  return (
<Page>
    <h1 zText="home.title">Zebflow</h1>
    <p zText="home.description">Composable automation runtime.</p>
    <button onClick="home.bump">Ping</button>
    <p>Visits: <span zText="home.visits">0</span></p>
</Page>
  );
}
