import { cx } from "zeb";
import Label from "@/components/ui/label";
import HelpTooltip from "@/components/ui/help-tooltip";

export default function Field(props) {
  return (
    <div className={cx("grid w-full items-center gap-1.5", props?.className)}>
      <div className="flex items-center gap-1.5">
        <Label label={props?.label} htmlFor={props?.id} />
        {props?.description && <HelpTooltip text={props.description} />}
      </div>
      {props.children}
    </div>
  );
}
