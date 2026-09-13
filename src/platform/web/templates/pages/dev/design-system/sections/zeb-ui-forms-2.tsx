import { Toggle } from "zeb/ui/toggle";
import { ToggleGroup, ToggleGroupItem } from "zeb/ui/toggle-group";
import { Slider } from "zeb/ui/slider";
import { InputOTP, InputOTPGroup, InputOTPSlot, InputOTPSeparator } from "zeb/ui/input-otp";
import { NativeSelect, NativeSelectOption, NativeSelectOptGroup } from "zeb/ui/native-select";
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupInput, InputGroupTextarea, InputGroupText } from "zeb/ui/input-group";
import { ButtonGroup, ButtonGroupText, ButtonGroupSeparator } from "zeb/ui/button-group";
import { Button } from "zeb/ui/button";
import { SectionHeading, Entry } from "@/pages/dev/design-system/components/gallery";

/**
 * zeb/ui — Forms, part 2: Toggle, ToggleGroup, Slider, InputOTP,
 * NativeSelect, InputGroup, ButtonGroup. Continued from zeb-ui-forms.tsx.
 */

export default function ZebUiForms2Section() {
  return (
    <div>
      <Entry
        name="Toggle"
        file="zeb/ui/toggle"
        description="A two-state button (aria-pressed). variant: default | outline. size: default | sm | lg."
        code={`import { Toggle } from "zeb/ui/toggle";

<Toggle defaultPressed>B</Toggle>
<Toggle variant="outline">I</Toggle>
<Toggle disabled>U</Toggle>
<Toggle aria-invalid>?</Toggle>`}
      >
        <div className="flex flex-wrap items-center gap-3">
          <Toggle defaultPressed aria-label="Bold">
            B
          </Toggle>
          <Toggle variant="outline" aria-label="Italic">
            I
          </Toggle>
          <Toggle size="sm" aria-label="Small">
            S
          </Toggle>
          <Toggle disabled aria-label="Disabled">
            D
          </Toggle>
          <Toggle aria-invalid aria-label="Invalid">
            !
          </Toggle>
        </div>
      </Entry>

      <Entry
        name="ToggleGroup"
        file="zeb/ui/toggle-group"
        description="A connected row of Toggles, composed through context like upstream. A roving-focus toolbar: ArrowLeft/Right move focus between items and wrap; Space/Enter/click toggles, same as a plain button."
        code={`import { ToggleGroup, ToggleGroupItem } from "zeb/ui/toggle-group";

<ToggleGroup type="single" defaultValue="center" variant="outline">
  <ToggleGroupItem value="left">Left</ToggleGroupItem>
  <ToggleGroupItem value="center">Center</ToggleGroupItem>
  <ToggleGroupItem value="right">Right</ToggleGroupItem>
</ToggleGroup>`}
      >
        <div className="flex flex-wrap items-center gap-6">
          <ToggleGroup type="single" defaultValue="center" variant="outline">
            <ToggleGroupItem value="left">Left</ToggleGroupItem>
            <ToggleGroupItem value="center">Center</ToggleGroupItem>
            <ToggleGroupItem value="right">Right</ToggleGroupItem>
          </ToggleGroup>
          <ToggleGroup type="multiple" defaultValue={["bold"]} disabled>
            <ToggleGroupItem value="bold">B</ToggleGroupItem>
            <ToggleGroupItem value="italic">I</ToggleGroupItem>
          </ToggleGroup>
        </div>
      </Entry>

      <Entry
        name="Slider"
        file="zeb/ui/slider"
        description="A single-thumb range (native input type=range underneath). Upstream's multi-thumb value array is dropped — one number in, one number out."
        code={`import { Slider } from "zeb/ui/slider";

<Slider defaultValue={40} />
<Slider defaultValue={70} disabled />`}
      >
        <div className="flex max-w-72 flex-col gap-6">
          <Slider defaultValue={40} />
          <Slider defaultValue={70} disabled />
        </div>
      </Entry>

      <Entry
        name="InputOTP"
        file="zeb/ui/input-otp"
        description="maxLength single-character inputs composed through context. Focus advances forward on entry, back on Backspace; ArrowLeft/Right move between slots; pasting a whole code distributes it across slots; the first slot offers autocomplete=one-time-code."
        code={`import { InputOTP, InputOTPGroup, InputOTPSlot, InputOTPSeparator } from "zeb/ui/input-otp";

<InputOTP maxLength={6}>
  <InputOTPGroup>
    <InputOTPSlot index={0} />
    <InputOTPSlot index={1} />
    <InputOTPSlot index={2} />
  </InputOTPGroup>
  <InputOTPSeparator />
  <InputOTPGroup>
    <InputOTPSlot index={3} />
    <InputOTPSlot index={4} />
    <InputOTPSlot index={5} />
  </InputOTPGroup>
</InputOTP>`}
      >
        <div className="flex flex-col gap-4">
          <InputOTP maxLength={6} defaultValue="12">
            <InputOTPGroup>
              <InputOTPSlot index={0} />
              <InputOTPSlot index={1} />
              <InputOTPSlot index={2} />
            </InputOTPGroup>
            <InputOTPSeparator />
            <InputOTPGroup>
              <InputOTPSlot index={3} />
              <InputOTPSlot index={4} />
              <InputOTPSlot index={5} />
            </InputOTPGroup>
          </InputOTP>
          <InputOTP maxLength={4} disabled>
            <InputOTPGroup>
              <InputOTPSlot index={0} />
              <InputOTPSlot index={1} />
              <InputOTPSlot index={2} />
              <InputOTPSlot index={3} />
            </InputOTPGroup>
          </InputOTP>
        </div>
      </Entry>

      <Entry
        name="NativeSelect"
        file="zeb/ui/native-select"
        description="A real <select> styled to match; option/optgroup only ever take OS Canvas/CanvasText colours in any browser."
        code={`import { NativeSelect, NativeSelectOption, NativeSelectOptGroup } from "zeb/ui/native-select";

<NativeSelect defaultValue="kiwi">
  <NativeSelectOptGroup label="Fruit">
    <NativeSelectOption value="apple">Apple</NativeSelectOption>
    <NativeSelectOption value="kiwi">Kiwi</NativeSelectOption>
  </NativeSelectOptGroup>
</NativeSelect>`}
      >
        <div className="flex flex-wrap items-center gap-4">
          <NativeSelect defaultValue="kiwi" className="max-w-48">
            <NativeSelectOptGroup label="Fruit">
              <NativeSelectOption value="apple">Apple</NativeSelectOption>
              <NativeSelectOption value="kiwi">Kiwi</NativeSelectOption>
            </NativeSelectOptGroup>
          </NativeSelect>
          <NativeSelect size="sm" disabled className="max-w-40">
            <NativeSelectOption>Disabled</NativeSelectOption>
          </NativeSelect>
          <NativeSelect aria-invalid className="max-w-40">
            <NativeSelectOption>Invalid</NativeSelectOption>
          </NativeSelect>
        </div>
      </Entry>

      <Entry
        name="InputGroup"
        file="zeb/ui/input-group"
        description="A bordered row combining an Input/Textarea with addons. multiline/invalid replace upstream's :has() detection as explicit props."
        code={`import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupInput, InputGroupText, InputGroupTextarea } from "zeb/ui/input-group";

<InputGroup>
  <InputGroupAddon><InputGroupText>$</InputGroupText></InputGroupAddon>
  <InputGroupInput placeholder="Amount" />
  <InputGroupAddon align="inline-end"><InputGroupButton>Max</InputGroupButton></InputGroupAddon>
</InputGroup>
<InputGroup multiline>
  <InputGroupTextarea placeholder="Message" />
</InputGroup>`}
      >
        <div className="flex max-w-80 flex-col gap-4">
          <InputGroup>
            <InputGroupAddon>
              <InputGroupText>$</InputGroupText>
            </InputGroupAddon>
            <InputGroupInput placeholder="Amount" />
            <InputGroupAddon align="inline-end">
              <InputGroupButton>Max</InputGroupButton>
            </InputGroupAddon>
          </InputGroup>
          <InputGroup multiline>
            <InputGroupTextarea placeholder="Message" />
          </InputGroup>
          <InputGroup invalid>
            <InputGroupInput placeholder="Invalid" aria-invalid />
          </InputGroup>
          <InputGroup>
            <InputGroupInput placeholder="Disabled" disabled />
          </InputGroup>
        </div>
      </Entry>

      <Entry
        name="ButtonGroup"
        file="zeb/ui/button-group"
        description="A row of controls sharing one border and squared inner corners. Corner classes are merged onto each child by position, not by :has()."
        code={`import { ButtonGroup, ButtonGroupText, ButtonGroupSeparator } from "zeb/ui/button-group";
import { Button } from "zeb/ui/button";

<ButtonGroup>
  <Button variant="outline">Left</Button>
  <Button variant="outline">Mid</Button>
  <Button variant="outline">Right</Button>
</ButtonGroup>`}
      >
        <div className="flex flex-wrap items-center gap-6">
          <ButtonGroup>
            <Button variant="outline">Left</Button>
            <Button variant="outline">Mid</Button>
            <Button variant="outline">Right</Button>
          </ButtonGroup>
          <ButtonGroup>
            <ButtonGroupText>Qty</ButtonGroupText>
            <Button variant="outline">-</Button>
            <ButtonGroupSeparator />
            <Button variant="outline">+</Button>
          </ButtonGroup>
          <ButtonGroup orientation="vertical">
            <Button variant="outline">Up</Button>
            <Button variant="outline">Down</Button>
          </ButtonGroup>
        </div>
      </Entry>
    </div>
  );
}
