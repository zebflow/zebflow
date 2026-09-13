import GalleryShell from "@/pages/dev/design-system/components/gallery-shell";
import TokensSection from "@/pages/dev/design-system/sections/tokens";
import ActionsSection from "@/pages/dev/design-system/sections/actions";
import FormsSection from "@/pages/dev/design-system/sections/forms";
import OverlaysSection from "@/pages/dev/design-system/sections/overlays";
import NavigationSection from "@/pages/dev/design-system/sections/navigation";
import DataSection from "@/pages/dev/design-system/sections/data";
import { initDesignSystemBehavior } from "@/pages/dev/design-system/design-system-behavior";

/**
 * The studio kit's gallery. One sidebar entry per group, one section file per
 * group, every primitive in `components/ui/` on exactly one of them — a test
 * in tests/rwe reads the `data-gallery-entry` markers and fails naming any
 * primitive that is missing. `zeb/ui` has its own page at /dev/design-system/ui.
 */

export const page = {
  head: {
    title: "Design System · Zebflow",
    description: "Platform UI reference for platform developers and agents",
  },
  html: { lang: "en" },
  body: { className: "font-sans" },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "Design System · Zebflow",
      description: input?.seo?.description ?? "Platform UI reference for platform developers and agents",
    },
  };
}

const GROUPS = [
  { id: "theme", label: "Theme", hint: "tokens" },
  { id: "actions", label: "Actions", hint: "button · toggle · menus" },
  { id: "forms", label: "Forms", hint: "input · select · pickers" },
  { id: "overlays", label: "Overlays", hint: "dialog · tooltip · toast" },
  { id: "navigation", label: "Navigation", hint: "tabs · trees" },
  { id: "data", label: "Data", hint: "card · table · badge" },
];

export default function Page(input) {
  const owner = input?.owner ?? "";
  const project = input?.project ?? null;
  initDesignSystemBehavior();

  return (
    <GalleryShell
      kit="/dev/design-system"
      title="Design System"
      groups={GROUPS}
      initial="theme"
      footnote="Every file in components/ui/ is on this page. Contract: docs/contracts/kinds/ui-theme."
    >
      {(group) => (
        <>
          <section hidden={group !== "theme"}><TokensSection /></section>
          <section hidden={group !== "actions"}><ActionsSection /></section>
          <section hidden={group !== "forms"}><FormsSection owner={owner} project={project} /></section>
          <section hidden={group !== "overlays"}><OverlaysSection /></section>
          <section hidden={group !== "navigation"}><NavigationSection owner={owner} project={project} /></section>
          <section hidden={group !== "data"}><DataSection /></section>
        </>
      )}
    </GalleryShell>
  );
}
