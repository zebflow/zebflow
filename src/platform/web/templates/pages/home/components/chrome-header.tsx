import { Link } from "zeb";
import Button from "@/components/ui/button";

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
        <Button as={Link} href="/profile" size="sm" variant="outline" className="rounded-md">
          Profile
        </Button>
        <form method="post" action="/logout">
          <Button type="submit" size="sm" variant="primary" className="rounded-md">
            Logout
          </Button>
        </form>
      </>
    );

  return (
    <nav className="fixed top-0 z-50 w-full border-b border-gray-200 bg-white/95 py-3 shadow-sm backdrop-blur-sm">
      <div className="mx-auto flex max-w-6xl items-center justify-between gap-4 px-6">
        <Link
          href="/home"
          className="flex items-center gap-3 text-xl font-bold tracking-tight text-gray-900 hover:no-underline"
        >
          <img src="/assets/branding/logo.svg" alt="Zebflow logo" className="h-9 w-9 shrink-0" />
          <span>
            ZEBFLOW <span className="ml-2 text-sm text-gray-400">Platform</span>
          </span>
        </Link>
        <div className="flex shrink-0 flex-wrap items-center justify-end gap-2">{trailing}</div>
      </div>
    </nav>
  );
}
