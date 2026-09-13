import { Input } from "zeb/ui/input";
import { Textarea } from "zeb/ui/textarea";
import { Label } from "zeb/ui/label";
import { Field, FieldLabel, FieldDescription, FieldError, FieldGroup, FieldSet, FieldLegend, FieldSeparator } from "zeb/ui/field";
import { Checkbox } from "zeb/ui/checkbox";
import { Switch } from "zeb/ui/switch";
import { RadioGroup, RadioGroupItem } from "zeb/ui/radio-group";
import { SectionHeading, Entry } from "@/pages/dev/design-system/components/gallery";

/**
 * zeb/ui — Forms, part 1: Input, Textarea, Label, Field, Checkbox, Switch,
 * RadioGroup. Continued in zeb-ui-forms-2.tsx (Toggle, ToggleGroup, Slider,
 * InputOTP, NativeSelect, InputGroup, ButtonGroup).
 */

export default function ZebUiFormsSection() {
  return (
    <div>
      <SectionHeading
        title="zeb/ui · Forms"
        description="Text entry, selection and grouping controls — native inputs styled to shadcn's spec, real keyboard and form behaviour underneath."
      />

      <Entry
        name="Input"
        file="zeb/ui/input"
        description="A native text input. Disabled and aria-invalid states shown alongside the default."
        code={`import { Input } from "zeb/ui/input";

<Input placeholder="Email" />
<Input placeholder="Disabled" disabled />
<Input placeholder="Invalid" aria-invalid />`}
      >
        <div className="flex flex-wrap gap-4">
          <Input placeholder="Email" className="max-w-56" />
          <Input placeholder="Disabled" disabled className="max-w-56" />
          <Input placeholder="Invalid" aria-invalid className="max-w-56" />
        </div>
      </Entry>

      <Entry
        name="Textarea"
        file="zeb/ui/textarea"
        description="Multi-line text. field-sizing-content (no static height) drops in favour of a fixed min-h-16."
        code={`import { Textarea } from "zeb/ui/textarea";

<Textarea placeholder="Notes" />
<Textarea placeholder="Disabled" disabled />
<Textarea placeholder="Invalid" aria-invalid />`}
      >
        <div className="flex flex-wrap gap-4">
          <Textarea placeholder="Notes" className="max-w-56" />
          <Textarea placeholder="Disabled" disabled className="max-w-56" />
          <Textarea placeholder="Invalid" aria-invalid className="max-w-56" />
        </div>
      </Entry>

      <Entry
        name="Label"
        file="zeb/ui/label"
        description="Pairs with a control via htmlFor/id, dims via peer-disabled when the paired control carries a peer class."
        code={`import { Label } from "zeb/ui/label";
import { Input } from "zeb/ui/input";

<Label htmlFor="email">Email</Label>
<Input id="email" />`}
      >
        <div className="flex flex-col gap-1.5 max-w-56">
          <Label htmlFor="zeb-ui-demo-email">Email</Label>
          <Input id="zeb-ui-demo-email" placeholder="you@example.com" />
        </div>
      </Entry>

      <Entry
        name="Field"
        file="zeb/ui/field"
        description="shadcn's Field/FieldLabel/FieldDescription/FieldError/FieldGroup/FieldSet/FieldLegend/FieldSeparator family, for laying out a form."
        code={`import { FieldSet, FieldLegend, FieldGroup, Field, FieldLabel, FieldDescription, FieldError, FieldSeparator } from "zeb/ui/field";
import { Input } from "zeb/ui/input";

<FieldSet>
  <FieldLegend>Account</FieldLegend>
  <FieldGroup>
    <Field>
      <FieldLabel htmlFor="username">Username</FieldLabel>
      <Input id="username" aria-invalid />
      <FieldError>Username is taken.</FieldError>
    </Field>
    <FieldSeparator>or</FieldSeparator>
    <Field>
      <FieldLabel htmlFor="bio">Bio</FieldLabel>
      <FieldDescription>Shown on your public profile.</FieldDescription>
    </Field>
  </FieldGroup>
</FieldSet>`}
        demoClassName="max-w-96"
      >
        <FieldSet>
          <FieldLegend>Account</FieldLegend>
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="zeb-ui-demo-username">Username</FieldLabel>
              <Input id="zeb-ui-demo-username" defaultValue="taken" aria-invalid />
              <FieldError>That username is taken.</FieldError>
            </Field>
            <FieldSeparator>or</FieldSeparator>
            <Field>
              <FieldLabel htmlFor="zeb-ui-demo-bio">Bio</FieldLabel>
              <Input id="zeb-ui-demo-bio" placeholder="A short bio" disabled />
              <FieldDescription>Shown on your public profile.</FieldDescription>
            </Field>
          </FieldGroup>
        </FieldSet>
      </Entry>

      <Entry
        name="Checkbox"
        file="zeb/ui/checkbox"
        description="A native checkbox (sr-only) inside a label, with a sibling span drawing the box. Checked, indeterminate, disabled and invalid states."
        code={`import { Checkbox } from "zeb/ui/checkbox";

<Checkbox defaultChecked />
<Checkbox />
<Checkbox checked="indeterminate" />
<Checkbox disabled />
<Checkbox aria-invalid />`}
      >
        <div className="flex flex-wrap items-center gap-6">
          <label className="flex items-center gap-2 text-sm text-foreground">
            <Checkbox defaultChecked /> Checked
          </label>
          <label className="flex items-center gap-2 text-sm text-foreground">
            <Checkbox /> Unchecked
          </label>
          <label className="flex items-center gap-2 text-sm text-foreground">
            <Checkbox checked="indeterminate" /> Indeterminate
          </label>
          <label className="flex items-center gap-2 text-sm text-muted-foreground">
            <Checkbox disabled defaultChecked /> Disabled
          </label>
          <label className="flex items-center gap-2 text-sm text-foreground">
            <Checkbox aria-invalid /> Invalid
          </label>
        </div>
      </Entry>

      <Entry
        name="Switch"
        file="zeb/ui/switch"
        description="An on/off button (role=switch). size accepts 'default' | 'sm'."
        code={`import { Switch } from "zeb/ui/switch";

<Switch defaultChecked />
<Switch size="sm" />
<Switch disabled defaultChecked />
<Switch aria-invalid />`}
      >
        <div className="flex flex-wrap items-center gap-6">
          <label className="flex items-center gap-2 text-sm text-foreground">
            <Switch defaultChecked /> On
          </label>
          <label className="flex items-center gap-2 text-sm text-foreground">
            <Switch size="sm" /> Small, off
          </label>
          <label className="flex items-center gap-2 text-sm text-muted-foreground">
            <Switch disabled defaultChecked /> Disabled
          </label>
          <label className="flex items-center gap-2 text-sm text-foreground">
            <Switch aria-invalid /> Invalid
          </label>
        </div>
      </Entry>

      <Entry
        name="RadioGroup"
        file="zeb/ui/radio-group"
        description="Mutually exclusive native radios, composed through context like upstream — an item is wired up no matter how deep it's nested. name defaults to useId(), so independent groups on one page never collide."
        code={`import { RadioGroup, RadioGroupItem } from "zeb/ui/radio-group";

<RadioGroup defaultValue="b">
  <label><RadioGroupItem value="a" /> Option A</label>
  <label><RadioGroupItem value="b" /> Option B</label>
  <label><RadioGroupItem value="c" disabled /> Option C (disabled)</label>
</RadioGroup>`}
      >
        <div className="flex flex-wrap gap-10">
          <RadioGroup defaultValue="b" className="gap-3">
            <label className="flex items-center gap-2 text-sm text-foreground">
              <RadioGroupItem value="a" /> Option A
            </label>
            <label className="flex items-center gap-2 text-sm text-foreground">
              <RadioGroupItem value="b" /> Option B
            </label>
            <label className="flex items-center gap-2 text-sm text-muted-foreground">
              <RadioGroupItem value="c" disabled /> Option C (disabled)
            </label>
            <label className="flex items-center gap-2 text-sm text-foreground">
              <RadioGroupItem value="d" aria-invalid /> Option D (invalid)
            </label>
          </RadioGroup>
          <RadioGroup defaultValue="y" className="gap-3">
            <label className="flex items-center gap-2 text-sm text-foreground">
              <RadioGroupItem value="x" /> A second, independent group
            </label>
            <label className="flex items-center gap-2 text-sm text-foreground">
              <RadioGroupItem value="y" /> picking here never clears the first
            </label>
          </RadioGroup>
        </div>
      </Entry>
    </div>
  );
}
