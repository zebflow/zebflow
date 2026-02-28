export const page = {
  head: {
    title: "Zebflow Visibility",
  },
  navigation: "history",
};

export const app = (() => {
return {
    state: {
      panel: {
        title: "Visibility Demo",
        open: true
      }
    },
    actions: {
      "panel.toggle": (ctx) => {
        const current = !!ctx.get("panel.open");
        ctx.set("panel.open", !current);
        return "panel.open";
      }
    }
  };
})();

export default function Page(input) {
  return (
<Page>
    <main>
      <h1 zText="panel.title">Visibility Demo</h1>
      <button onClick="panel.toggle">Toggle Panel</button>
      <section zShow="panel.open">
        <p>This panel is visible.</p>
      </section>
      <section zHide="panel.open">
        <p>This panel is hidden.</p>
      </section>
    </main>
</Page>
  );
}
