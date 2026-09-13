import GalleryShell from "@/pages/dev/design-system/components/gallery-shell";
import ZebUiActionsSection from "@/pages/dev/design-system/sections/zeb-ui-actions";
import ZebUiFormsSection from "@/pages/dev/design-system/sections/zeb-ui-forms";
import ZebUiForms2Section from "@/pages/dev/design-system/sections/zeb-ui-forms-2";
import ZebUiDisplaySection from "@/pages/dev/design-system/sections/zeb-ui-display";
import ZebUiDisplay2Section from "@/pages/dev/design-system/sections/zeb-ui-display-2";
import ZebUiDisclosureSection from "@/pages/dev/design-system/sections/zeb-ui-disclosure";
import ZebUiOverlaysSection from "@/pages/dev/design-system/sections/zeb-ui-overlays";
import ZebUiOverlays2Section from "@/pages/dev/design-system/sections/zeb-ui-overlays-2";
import ZebUiEditorSection from "@/pages/dev/design-system/sections/zeb-ui-editor";
import { initDesignSystemBehavior } from "@/pages/dev/design-system/design-system-behavior";

/**
 * The `zeb/ui` gallery: the library's own source, inlined by the compiler
 * exactly as a project page gets it, rendered on the platform theme. Every
 * component file must appear here — tests/rwe/zeb_ui.rs reads the
 * `file="zeb/ui/<name>"` markers.
 */

export const page = {
  head: {
    title: "zeb/ui · Zebflow",
    description: "The component library project pages import — every component, every variant",
  },
  html: { lang: "en" },
  body: { className: "font-sans" },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "zeb/ui · Zebflow",
      description: input?.seo?.description ?? "The component library project pages import",
    },
  };
}

const GROUPS = [
  { id: "actions", label: "Actions", hint: "button" },
  { id: "forms", label: "Forms", hint: "input · checkbox · switch · slider" },
  { id: "display", label: "Display", hint: "card · badge · table · avatar" },
  { id: "disclosure", label: "Disclosure", hint: "tabs · accordion · resizable" },
  { id: "overlays", label: "Overlays", hint: "dialog · menus · select · toast" },
  { id: "editor", label: "Editor", hint: "notion-style blocks" },
];

export default function Page() {
  initDesignSystemBehavior();

  return (
    <GalleryShell
      kit="/dev/design-system/ui"
      title="zeb/ui"
      groups={GROUPS}
      initial="actions"
      footnote={'import { Button } from "zeb/ui/button" — no install. Clone one to own it: install_ui_components names=["button"].'}
    >
      {(group) => (
        <>
          <section hidden={group !== "actions"}><ZebUiActionsSection /></section>
          <section hidden={group !== "forms"}><ZebUiFormsSection /><ZebUiForms2Section /></section>
          <section hidden={group !== "display"}><ZebUiDisplaySection /><ZebUiDisplay2Section /></section>
          <section hidden={group !== "disclosure"}><ZebUiDisclosureSection /></section>
          <section hidden={group !== "overlays"}><ZebUiOverlaysSection /><ZebUiOverlays2Section /></section>
          <section hidden={group !== "editor"}><ZebUiEditorSection /></section>
        </>
      )}
    </GalleryShell>
  );
}
