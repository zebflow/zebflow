import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";

/** The Hub page's tabs. Each tab is a link the server marks active. */
export default function HubTabBar({ tabs }) {
  return (
    <StudioTabNav>
      {tabs.map((item, index) => (
        <StudioTabLink key={`${item?.href ?? "tab"}-${index}`} href={item?.href ?? "#"} active={item?.classes === "is-active"}>
          {item?.label}
        </StudioTabLink>
      ))}
    </StudioTabNav>
  );
}
