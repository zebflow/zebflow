import { cx, useState } from "zeb/react";
import Badge from "@/components/ui/badge";
import Button from "@/components/ui/button";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import CardDescription from "@/components/ui/card-description";
import Input from "@/components/ui/input";
import Separator from "@/components/ui/separator";

/** Every pipeline node the running platform knows about. */
export default function NodeRegistryPanel({ groups, count }) {
  const [searchQuery, setSearchQuery] = useState("");
  const [activeTab, setActiveTab] = useState("installed");

  const nodeGroups = Array.isArray(groups) ? groups : [];
  const query = searchQuery.toLowerCase().trim();

  const filteredGroups = query
    ? nodeGroups
        .map((group) => ({
          ...group,
          nodes: (Array.isArray(group?.nodes) ? group.nodes : []).filter((node) =>
            `${node?.title ?? ""} ${node?.kind ?? ""} ${node?.description ?? ""}`
              .toLowerCase()
              .includes(query)
          ),
        }))
        .filter((group) => group.nodes.length > 0)
    : nodeGroups;

  const visibleCount = filteredGroups.reduce(
    (sum, g) => sum + (g.nodes?.length ?? 0),
    0
  );

  return (
    <div className="border border-border rounded-lg bg-surface overflow-hidden">
      {/* Toolbar */}
      <div className="flex flex-row items-center gap-[0.55rem] px-3 pt-[0.7rem] pb-[0.6rem]">
        <Input
          placeholder="Search nodes by name or kind..."
          value={searchQuery}
          onInput={(e) => setSearchQuery(e.currentTarget.value)}
        />
        <Button variant="outline" size="sm" label="+ Install" disabled />
      </div>

      {/* Tabs */}
      <div className="flex gap-1 px-3 pb-2">
        {(["installed", "discover", "updates"] as const).map((tab) => (
          <Button
            key={tab}
            variant="ghost"
            size="sm"
            className={cx(tab === activeTab && "bg-accent/10 text-accent border-accent/40")}
            label={tab === "installed" ? `Installed · ${count}` : tab === "discover" ? "Discover" : "Updates"}
            onClick={() => setActiveTab(tab)}
          />
        ))}
      </div>

      <Separator />

      {/* Installed panel */}
      {activeTab === "installed" ? (
        <div>
          <p className="px-3 py-[0.4rem] text-[0.72rem] text-body-soft border-b border-border-soft">
            {visibleCount === count
              ? `${count} nodes · ${count} built-in`
              : `${visibleCount} of ${count} nodes · ${count} built-in`}
          </p>
          <div className="flex flex-col gap-[0.35rem] px-3 py-[0.6rem]">
            {filteredGroups.length === 0 ? (
              <p className="p-8 text-center text-[0.8rem] text-body-soft">No nodes found.</p>
            ) : (
              filteredGroups.map((group, gi) => (
                <div key={`grp-${gi}`}>
                  <div className="flex items-center gap-2 mb-[0.35rem] mt-2 first:mt-0">
                    {group?.prefix ? (
                      <span className="text-[0.65rem] font-mono text-body-soft tracking-[0.05em] whitespace-nowrap shrink-0">{group.prefix}</span>
                    ) : null}
                    <div className="flex-1 h-px bg-border-soft" />
                  </div>
                  {(Array.isArray(group?.nodes) ? group.nodes : []).map((node, ni) => (
                    <div
                      key={`${node?.kind ?? "node"}-${ni}`}
                      className="flex items-stretch border border-border-soft rounded-lg bg-surface-2 overflow-hidden transition-colors duration-[120ms]"
                    >
                      <div className="w-[3px] shrink-0 bg-border" />
                      <div className="flex-1 min-w-0 px-3 py-[0.6rem] flex items-start justify-between gap-3">
                        <div className="flex-1 min-w-0">
                          <div className="text-[0.83rem] font-bold text-body leading-tight">{node?.title}</div>
                          <div className="text-[0.66rem] font-mono text-body-soft mt-[0.15rem] tracking-[0.03em]">{node?.kind}</div>
                          <div className="text-[0.75rem] leading-[1.4] text-body-soft mt-[0.3rem]">{node?.description}</div>
                        </div>
                        <div className="flex items-center flex-wrap gap-[0.3rem] shrink-0 pt-[0.1rem]">
                          {node?.ai_registered ? (
                            <Badge label="agent tool" variant="outline" className="text-[0.65rem] text-gray-200 border-white/25 bg-transparent" />
                          ) : null}
                          <span className="inline-flex items-center px-2 py-[0.25rem] rounded-full border border-[rgba(74,222,128,0.3)] text-[#4ade80] text-[0.65rem] font-mono uppercase tracking-widest">● installed</span>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              ))
            )}
          </div>
        </div>
      ) : null}

      {/* Discover panel */}
      {activeTab === "discover" ? (
        <div className="p-4 px-3">
          <Card>
            <CardContent>
              <CardDescription label="Community registry — coming soon." />
              <CardDescription label="Install custom nodes from a Git URL using + Install above." />
            </CardContent>
          </Card>
        </div>
      ) : null}

      {/* Updates panel */}
      {activeTab === "updates" ? (
        <div className="p-4 px-3">
          <Card>
            <CardContent>
              <CardDescription label={`All ${count} built-in nodes are current.`} />
            </CardContent>
          </Card>
        </div>
      ) : null}
    </div>
  );
}
