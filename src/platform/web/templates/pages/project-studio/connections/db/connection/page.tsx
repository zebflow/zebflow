import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import { useConnectionWorkspace } from "@/components/db/use-connection-workspace";
import ConnectionContent from "@/pages/project-studio/connections/db/connection/components/connection-content";
import ConnectionDialogs from "@/pages/project-studio/connections/db/connection/components/connection-dialogs";

export const page = {
  head: {
    links: [
      { rel: "stylesheet", href: "/assets/platform/db-suite.css" },
      { rel: "stylesheet", href: "/assets/platform/devicons.css" },
    ],
  },
  html: {
    lang: "en",
  },
  body: {
    className: "font-sans",
  },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "",
      description: input?.seo?.description ?? "",
    },
  };
}

/**
 * Layout only: the tab nav, the current tab's content, and the dialogs that
 * can open over it. `useConnectionWorkspace` is the composition root — every
 * child below reads off the one `workspace` object it returns rather than
 * being handed a dozen individual props at this seam.
 */
export default function Page(input) {
  const workspace = useConnectionWorkspace(input);
  const { navLinks, suiteTabs, connection } = workspace;

  return (
    <ProjectStudioShell
      projectHref={input.project_href}
      projectLabel={input.title}
      currentMenu={`Databases / ${connection.slug || "connection"}`}
      owner={input.owner}
      project={input.project}
      nav={input.nav}
    >
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <StudioTabNav>
          <StudioTabLink href={navLinks.db_connections ?? "#"}>Connections</StudioTabLink>
          {suiteTabs.map((item, index) => (
            <StudioTabLink key={`${item?.href ?? "tab"}-${index}`} href={item?.href ?? "#"} active={item?.classes === "is-active"}>
              {item?.label}
            </StudioTabLink>
          ))}
        </StudioTabNav>

        <ConnectionContent input={input} workspace={workspace} />
      </div>
      <ConnectionDialogs workspace={workspace} />
    </ProjectStudioShell>
  );
}
