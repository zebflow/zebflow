import UnifiedRegistryEditor from "@/pages/project-studio/pipelines/registry/components/unified-registry-editor";

export const page = {
  head: {
    title: ctx?.seo?.title ?? "",
    description: ctx?.seo?.description ?? "",
    links: [
      { rel: "stylesheet", href: "/assets/libraries/zeb/icons/0.1/runtime/devicons.css" },
    ],
  },
  html: { lang: "en" },
  body: { className: "font-sans" },
  navigation: "history",
};

export default function Page(input) {
  return <UnifiedRegistryEditor {...input} />;
}
