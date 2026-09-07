import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";

/**
 * Projects this person has been asked to join.
 *
 * Sits on the home page because that is where someone lands, and until they
 * answer they cannot reach the project the invitation is for — a notice inside
 * it would be behind the very door it opens.
 *
 * Renders nothing when there is nothing waiting. An empty panel saying "no
 * invitations" is a permanent reminder of an occasional event.
 */
export default function PendingInvitations() {
  const [invites, setInvites] = useState([]);
  const [busy, setBusy] = useState("");
  const [problem, setProblem] = useState("");

  async function load() {
    try {
      const res = await requestJson("/api/invites");
      setInvites(Array.isArray(res?.items) ? res.items : []);
    } catch (err) {
      setProblem(String(err?.message || err));
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function answer(invite, verb) {
    setBusy(invite.invite_id);
    setProblem("");
    try {
      await requestJson(
        `/api/invites/${encodeURIComponent(invite.owner)}/${encodeURIComponent(invite.project)}/${encodeURIComponent(invite.invite_id)}/${verb}`,
        { method: "POST" },
      );
      if (verb === "accept") {
        // The project is reachable now, and the list it belongs in is rendered
        // from the server. Re-reading the page is the honest way to show both.
        window.location.reload();
        return;
      }
      await load();
    } catch (err) {
      setProblem(String(err?.message || err));
    } finally {
      setBusy("");
    }
  }

  if (!invites.length) return null;

  return (
    <section data-pending-invitations className="flex flex-col gap-2">
      <p className="text-[0.74rem] uppercase tracking-[0.08em] text-body-soft">
        You have been invited
      </p>
      {problem ? <p className="text-[0.78rem] text-red-400">{problem}</p> : null}
      {invites.map((invite) => (
        <div
          key={invite.invite_id}
          data-invite={invite.invite_id}
          className="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border px-3 py-2"
        >
          <div>
            <p className="text-[0.85rem] font-medium">
              {invite.owner}/{invite.project}
            </p>
            <p className="text-[0.76rem] text-body-soft">
              as {invite.role_preset} · invited by {invite.invited_by}
              {invite.note ? ` · ${invite.note}` : ""}
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={!!busy}
              onClick={() => answer(invite, "decline")}
            >
              Decline
            </Button>
            <Button
              type="button"
              size="sm"
              disabled={!!busy}
              onClick={() => answer(invite, "accept")}
            >
              {busy === invite.invite_id ? "Joining…" : "Accept"}
            </Button>
          </div>
        </div>
      ))}
    </section>
  );
}
