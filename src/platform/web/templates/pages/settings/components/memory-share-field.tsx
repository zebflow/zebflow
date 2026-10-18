import Field from "@/components/ui/field";
import Input from "@/components/ui/input";

/**
 * One memory share of the instance: what is written (`auto` or a size with
 * its unit) and what it comes to on this office.
 */
export default function MemoryShareField(props) {
  return (
    <Field label={props.label} id={props.id} description={props.description}>
      <Input
        id={props.id}
        value={props.value}
        onInput={(event) => props.onChange(event.currentTarget.value)}
        placeholder="auto"
        autoComplete="off"
        required
      />
      <p className="text-xs text-gray-500">{props.comesTo}</p>
    </Field>
  );
}
