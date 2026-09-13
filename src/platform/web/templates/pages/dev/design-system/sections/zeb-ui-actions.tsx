import { Button } from "zeb/ui/button";
import { SectionHeading, Entry, Matrix } from "@/pages/dev/design-system/components/gallery";

/**
 * zeb/ui — the component library a project imports without installing.
 * These entries render the library's own source, inlined by the compiler,
 * on the studio's theme: what a user page gets, judged next to the studio's
 * primitives. `file="zeb/ui/<name>"` is what the guard reads.
 */

const VARIANTS = ["default", "secondary", "outline", "ghost", "destructive", "link"];
const SIZES = ["lg", "default", "sm", "xs", "icon", "icon-sm", "icon-xs"];

export default function ZebUiActionsSection() {
  return (
    <div>
      <SectionHeading
        title="zeb/ui · Actions"
        description={'import { Button } from "zeb/ui/button" — shadcn\'s API on Zebflow\'s engine. No install; clone the file to own it.'}
      />

      <Entry
        name="Button"
        file="zeb/ui/button"
        description="shadcn's variants and sizes, verbatim. as='a' replaces asChild."
        code={`import { Button } from "zeb/ui/button";

<Button>Save</Button>
<Button variant="outline" size="sm">Cancel</Button>
<Button variant="destructive" size="icon" aria-label="Delete">✕</Button>
<Button as="a" href="/home" variant="link">Home</Button>`}
      >
        <Matrix
          rows={VARIANTS}
          cols={SIZES}
          render={(variant, size) => (
            <Button variant={variant} size={size} aria-label={size.startsWith("icon") ? variant : undefined}>
              {size.startsWith("icon") ? "✓" : "Button"}
            </Button>
          )}
        />
        <div className="mt-6 flex flex-wrap items-center gap-3 border-t border-border pt-5">
          <Button disabled>Disabled</Button>
          <Button variant="outline" disabled>Disabled</Button>
          <Button as="a" href="#" variant="link">as an anchor</Button>
        </div>
      </Entry>
    </div>
  );
}
