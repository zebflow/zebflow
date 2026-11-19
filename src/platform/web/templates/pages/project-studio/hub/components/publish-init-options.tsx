import CheckboxField from "@/components/ui/checkbox-field";
import InitialDataList from "@/pages/project-studio/hub/components/initial-data-list";
import InstalledLibrariesList from "@/pages/project-studio/hub/components/installed-libraries-list";

/**
 * What a project bundle carries for the installing project to initialize from:
 * schemas, initial data and libraries. `publish` is `usePublishForm()`.
 */
export default function PublishInitOptions({ publish }) {
  const publishForm = publish.form;
  const patchPublishForm = publish.patch;
  const publishOptions = publish.options;
  const { availableLibraries, sekejapSchemaAvailable, sqliteSchemaAvailable, initialDataItems, initialDataAvailable } = publish.facts;

  return (
    <div className="rounded-lg border border-border bg-accent/20 p-3 space-y-3">
      <div>
        <p className="m-0 text-sm font-medium text-foreground">Project bundle initialization</p>
        <p className="m-0 mt-1 text-xs text-muted-foreground">Choose the schema and runtime settings that the installed project should initialize from.</p>
      </div>
      <CheckboxField
        label="Include Sekejap schema"
        description={
          sekejapSchemaAvailable
            ? `${publishOptions?.sekejap_schema?.table_count || 0} managed table schema(s)`
            : "No Sekejap schema found in this project"
        }
        checked={publishForm.include_sekejap_schema}
        disabled={!sekejapSchemaAvailable}
        onChange={(e) => patchPublishForm({ include_sekejap_schema: e.target.checked })}
      />
      <CheckboxField
        label="Include SQLite schema"
        description={
          sqliteSchemaAvailable
            ? "Local SQLite DDL will be exported as schemas/sqlite/schema.sql"
            : "No SQLite schema found in local.db"
        }
        checked={publishForm.include_sqlite_schema}
        disabled={!sqliteSchemaAvailable}
        onChange={(e) => patchPublishForm({ include_sqlite_schema: e.target.checked })}
      />
      <InitialDataList
        items={initialDataItems}
        available={initialDataAvailable}
        form={publishForm}
        patch={patchPublishForm}
        onToggle={publish.toggleInitialDataPath}
      />
      <InstalledLibrariesList
        libraries={availableLibraries}
        selected={publishForm.include_libraries}
        onToggle={publish.togglePublishLibrary}
      />
    </div>
  );
}
