import { useState } from "zeb/react";
import Input from "@/components/ui/input";
import Textarea from "@/components/ui/textarea";
import { Select, SelectOption } from "@/components/ui/select";
import Checkbox from "@/components/ui/checkbox";
import CheckboxField from "@/components/ui/checkbox-field";
import Label from "@/components/ui/label";
import Field from "@/components/ui/field";
import FolderPicker from "@/components/ui/folder-picker";
import MapPicker from "@/components/ui/map-picker";
import TraceCaptureFields from "@/components/ui/trace-capture-fields";
import Button from "@/components/ui/button";
import { SectionHeading, Entry } from "@/pages/dev/design-system/components/gallery";

export default function FormsSection({ owner, project }) {
  const [region, setRegion] = useState("apac");
  const [agree, setAgree] = useState(true);
  const [folder, setFolder] = useState("pipelines");
  const [mapOpen, setMapOpen] = useState(false);
  const [geometry, setGeometry] = useState("");
  const [trace, setTrace] = useState({});

  return (
    <div>
      <SectionHeading title="Forms" description="Controls that take a value. Every one paints on bg-popover with a border-input hairline and a ring-ring focus." />

      <Entry
        name="Input"
        file="input.tsx"
        description="type, placeholder, disabled, readOnly, min/max/step, and the usual events."
        code={`<Input placeholder="jane_doe" />
<Input type="number" min={0} max={10} defaultValue={3} />
<Input disabled value="disabled" />
<Input readOnly value="read only" />`}
      >
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Input placeholder="jane_doe" />
          <Input type="number" min={0} max={10} defaultValue={3} />
          <Input disabled value="disabled" />
          <Input readOnly value="read only" />
          <Input type="password" defaultValue="hunter2" />
          <Input type="date" />
        </div>
      </Entry>

      <Entry
        name="Textarea"
        file="textarea.tsx"
        description="Multi-line. `rows` sets the initial height; it resizes vertically."
        code={`<Textarea rows={4} placeholder="Why do you want access?" />`}
      >
        <Textarea rows={4} placeholder="Why do you want access?" />
      </Entry>

      <Entry
        name="Select"
        file="select.tsx"
        description="A native <select>. Controlled through `value` — the matching SelectOption is marked selected on the server, so the right one shows before hydration."
        code={`<Select value={region} onChange={(e) => setRegion(e.target.value)}>
  <SelectOption value="apac" label="Asia-Pacific" />
  <SelectOption value="emea" label="Europe" />
</Select>`}
      >
        <div className="flex flex-wrap items-center gap-4">
          <Select value={region} onChange={(e) => setRegion(e.target.value)}>
            <SelectOption value="apac" label="Asia-Pacific" />
            <SelectOption value="emea" label="Europe, Middle East, Africa" />
            <SelectOption value="amer" label="Americas" />
          </Select>
          <span className="font-mono text-xs text-muted-foreground">value: {region}</span>
        </div>
      </Entry>

      <Entry
        name="Checkbox"
        file="checkbox.tsx"
        description="Compact, mono label — for toolbars and consoles. Forwards data-* attributes to the input."
        code={`<Checkbox label="High" data-assistant-use-high />`}
      >
        <div className="flex flex-wrap items-center gap-6">
          <Checkbox label="High" defaultChecked />
          <Checkbox label="Auto nav" />
          <Checkbox label="Disabled" disabled />
        </div>
      </Entry>

      <Entry
        name="CheckboxField"
        file="checkbox-field.tsx"
        description="A checkbox with a sentence and an optional description — for settings forms."
        code={`<CheckboxField
  label="Accept new members automatically"
  description="Skips the approval queue. Anyone with the link can join."
  checked={agree}
  onChange={(e) => setAgree(e.target.checked)}
/>`}
      >
        <div className="grid gap-4">
          <CheckboxField
            label="Accept new members automatically"
            description="Skips the approval queue. Anyone with the link can join."
            checked={agree}
            onChange={(e) => setAgree(e.target.checked)}
          />
          <CheckboxField label="A disabled option" description="Not available on this plan." disabled />
        </div>
      </Entry>

      <Entry
        name="Label · Field"
        file="field.tsx"
        description="Label is the caption style on its own. Field is Label + optional HelpTooltip + the control, stacked."
        code={`<Field label="Username" id="u" description="Letters and numbers only.">
  <Input id="u" placeholder="jane_doe" />
</Field>`}
      >
        <div className="grid grid-cols-1 gap-6 sm:grid-cols-2">
          <div className="grid gap-2">
            <Label label="A bare label" htmlFor="bare" />
            <Input id="bare" placeholder="the control it captions" />
          </div>
          <Field label="Username" id="ds-username" description="Letters and numbers only.">
            <Input id="ds-username" placeholder="jane_doe" />
          </Field>
          <Field label="Region" id="ds-region">
            <Select id="ds-region" value={region} onChange={(e) => setRegion(e.target.value)}>
              <SelectOption value="apac" label="Asia-Pacific" />
              <SelectOption value="emea" label="Europe" />
              <SelectOption value="amer" label="Americas" />
            </Select>
          </Field>
          <Field label="Notes" id="ds-notes" description="Shown to admins only.">
            <Textarea id="ds-notes" rows={2} />
          </Field>
        </div>
      </Entry>

      <Entry
        name="TraceCaptureFields"
        file="trace-capture-fields.tsx"
        description="The five capture limits, as a controlled fieldset. `scope` project | pipeline changes the blank-field hint."
        code={`<TraceCaptureFields values={trace} onChange={setTrace} defaults={{}} scope="pipeline" />`}
      >
        <TraceCaptureFields values={trace} onChange={setTrace} defaults={{}} scope="pipeline" />
      </Entry>

      <Entry
        name="FolderPicker"
        file="folder-picker.tsx"
        description={project ? `Live against ${owner}/${project}. Folders load as they open.` : "Needs a project; the viewer has none yet."}
        code={`<FolderPicker owner={owner} project={project} value={folder} onChange={setFolder} rootLabel="repo" />`}
      >
        {project ? (
          <div className="grid gap-3">
            <FolderPicker owner={owner} project={project} value={folder} onChange={setFolder} rootLabel="repo" />
            <span className="font-mono text-xs text-muted-foreground">value: {folder || "(root)"}</span>
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">No project to browse.</p>
        )}
      </Entry>

      <Entry
        name="MapPicker"
        file="map-picker.tsx"
        description="A dialog for picking a point, line or polygon. Returns GeoJSON-ish text through onSave."
        code={`<MapPicker open={open} onOpenChange={setOpen} value={geometry} onSave={setGeometry} onClear={() => setGeometry("")} />`}
      >
        <div className="flex flex-wrap items-center gap-4">
          <Button variant="outline" onClick={() => setMapOpen(true)}>Pick geometry…</Button>
          <span className="max-w-md truncate font-mono text-xs text-muted-foreground">{geometry || "(nothing picked)"}</span>
          <MapPicker
            open={mapOpen}
            onOpenChange={setMapOpen}
            value={geometry}
            onSave={(v) => { setGeometry(typeof v === "string" ? v : JSON.stringify(v)); setMapOpen(false); }}
            onClear={() => setGeometry("")}
          />
        </div>
      </Entry>
    </div>
  );
}
