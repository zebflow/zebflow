import { Link } from "zeb/react";
import Button from "@/components/ui/button";

function ProfileIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="h-[18px] w-[18px]">
      <circle cx="12" cy="8" r="4" stroke="currentColor" strokeWidth="1.8" />
      <path d="M5 20a7 7 0 0 1 14 0" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
    </svg>
  );
}

/**
 * Fixed top bar for authenticated platform pages outside the project studio.
 * Home uses the hub dialog directly on `/home`.
 *
 * Why “chrome”: in UI jargon, *chrome* is the framing around the content window
 * (nav, toolbars, system trim) — the same sense as in “browser chrome”, not “Google Chrome”.
 */
export default function ChromeHeader(props) {
  const trailing =
    props?.children != null ? (
      props.children
    ) : (
      <>
        <Button
          as={Link}
          href="/profile"
          size="icon"
          variant="outline"
          className="rounded-md border-[#c46255] bg-[#c46255] text-white hover:bg-[#ad5149]"
          aria-label="Profile"
          title="Profile"
        >
          <ProfileIcon />
        </Button>
        <form method="post" action="/logout">
          <Button
            type="submit"
            size="sm"
            variant="primary"
            className="rounded-md border-[#ed752e] bg-[#ed752e] text-white hover:bg-[#f6863c]"
          >
            Logout
          </Button>
        </form>
      </>
    );

  return (
    <nav className="fixed top-0 z-50 w-full border-b border-border bg-popover/95 py-3 shadow-sm backdrop-blur-sm">
      <div className="mx-auto flex w-full max-w-[1960px] items-center justify-between gap-4 px-6 sm:px-10">
        <Link
          href="/home"
          className="flex items-center gap-3 text-lg font-semibold tracking-tight text-foreground hover:no-underline"
        >
          <img src="/assets/branding/logo.svg" alt="Zebflow logo" className="h-9 w-9 shrink-0" />
          <span className="flex items-baseline gap-2">
            <span>zebflow</span>
            <span className="text-xs font-medium text-muted-foreground">Platform</span>
          </span>
        </Link>
        <div className="flex shrink-0 flex-wrap items-center justify-end gap-2">{trailing}</div>
      </div>
    </nav>
  );
}
