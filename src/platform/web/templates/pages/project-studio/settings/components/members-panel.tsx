import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";
import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import ConfirmDialog from "@/components/ui/confirm-dialog";
import { settingsStatusToneClass } from "@/pages/project-studio/settings/components/settings-lib";
import SettingsSection from "@/pages/project-studio/settings/components/settings-section";

/**
 * Who works on this project, and who has been asked to.
 *
 * Invited and accepted, never added outright: a member's git identity ends up
 * on commits made here, so joining is something a person agrees to rather than
 * something done to them. Until they answer, the invitation is all there is —
 * which is why pending invitations are listed beside the members rather than
 * hidden behind a tab.
 */
export default function MembersPanel({ membersApi, invitesApi, grantableRoles }) {
  const [members, setMembers] = useState([]);
  const [invites, setInvites] = useState([]);
  const [targetUser, setTargetUser] = useState("");
  const [role, setRole] = useState(grantableRoles?.[0]?.key ?? "guest");
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);
  const [statusMsg, setStatusMsg] = useState("");
  const [statusTone, setStatusTone] = useState("info");
  const [removing, setRemoving] = useState(null);

  const roles = Array.isArray(grantableRoles) ? grantableRoles : [];

  function say(message, tone) {
    setStatusMsg(message);
    setStatusTone(tone || "info");
  }

  async function load() {
    try {
      const [memberRes, inviteRes] = await Promise.all([
        requestJson(membersApi),
        requestJson(invitesApi),
      ]);
      setMembers(Array.isArray(memberRes?.items) ? memberRes.items : []);
      // Only invitations still waiting on an answer. A revoked or accepted one
      // is history, and history belongs in the log rather than a to-do list.
      setInvites(
        (Array.isArray(inviteRes?.items) ? inviteRes.items : []).filter(
          (invite) => invite?.status === "pending",
        ),
      );
    } catch (err) {
      say(String(err?.message || err), "error");
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function invite(event) {
    event.preventDefault();
    const user = targetUser.trim();
    if (!user) {
      say("Name the person to invite.", "error");
      return;
    }
    setBusy(true);
    try {
      await requestJson(invitesApi, {
        method: "POST",
        body: JSON.stringify({ target_user: user, role_preset: role, note: note.trim() }),
      });
      setTargetUser("");
      setNote("");
      await load();
      say(`Invited ${user}. They join when they accept.`, "ok");
    } catch (err) {
      say(String(err?.message || err), "error");
    } finally {
      setBusy(false);
    }
  }

  async function revoke(invite) {
    setBusy(true);
    try {
      await requestJson(`${invitesApi}/${encodeURIComponent(invite.invite_id)}`, {
        method: "DELETE",
      });
      await load();
      say(`Invitation to ${invite.target_user} withdrawn.`, "ok");
    } catch (err) {
      say(String(err?.message || err), "error");
    } finally {
      setBusy(false);
    }
  }

  async function changeRole(member, nextRole) {
    setBusy(true);
    try {
      await requestJson(membersApi, {
        method: "POST",
        body: JSON.stringify({ user_id: member.user_id, role_preset: nextRole }),
      });
      await load();
      say(`${member.user_id} is now ${nextRole}.`, "ok");
    } catch (err) {
      say(String(err?.message || err), "error");
    } finally {
      setBusy(false);
    }
  }

  async function removeMember(member) {
    setBusy(true);
    try {
      await requestJson(`${membersApi}/${encodeURIComponent(member.user_id)}`, {
        method: "DELETE",
      });
      await load();
      say(`${member.user_id} removed from the project.`, "ok");
    } catch (err) {
      say(String(err?.message || err), "error");
    } finally {
      setBusy(false);
    }
  }

  return (
    <SettingsSection
      title="Members"
      description="Who works on this project and what each of them may do."
    >
      <form onSubmit={invite} className="flex flex-wrap items-end gap-3">
        <Field label="Invite" id="member-invite-user" className="min-w-[12rem] flex-1">
          <Input
            id="member-invite-user"
            value={targetUser}
            placeholder="username"
            onInput={(e) => setTargetUser(e.currentTarget.value)}
          />
        </Field>
        <Field label="As" id="member-invite-role">
          <Select id="member-invite-role" value={role} onChange={(e) => setRole(e.currentTarget.value)}>
            {roles.map((entry) => (
              <SelectOption key={entry.key} value={entry.key}>
                {entry.title}
              </SelectOption>
            ))}
          </Select>
        </Field>
        <Field label="Note" id="member-invite-note" className="min-w-[12rem] flex-1">
          <Input
            id="member-invite-note"
            value={note}
            placeholder="why, so they know what they are agreeing to"
            onInput={(e) => setNote(e.currentTarget.value)}
          />
        </Field>
        <Button type="submit" disabled={busy}>
          {busy ? "Working…" : "Send invitation"}
        </Button>
      </form>

      {statusMsg ? (
        <p className={`text-[0.78rem] ${settingsStatusToneClass(statusTone)}`}>{statusMsg}</p>
      ) : null}

      <StudioTable>
        <StudioThead>
          <StudioTh>Member</StudioTh>
          <StudioTh>Role</StudioTh>
          <StudioTh>Added by</StudioTh>
          <StudioTh> </StudioTh>
        </StudioThead>
        <tbody>
          {members.map((member) => (
            <tr key={member.user_id} data-member={member.user_id}>
              <StudioTd>{member.user_id}</StudioTd>
              <StudioTd>
                <Select
                  value={member.role_preset}
                  disabled={busy || !roles.some((entry) => entry.key === member.role_preset)}
                  onChange={(e) => changeRole(member, e.currentTarget.value)}
                >
                  {roles.map((entry) => (
                    <SelectOption key={entry.key} value={entry.key}>
                      {entry.title}
                    </SelectOption>
                  ))}
                </Select>
              </StudioTd>
              <StudioTd>{member.created_by || "—"}</StudioTd>
              <StudioTd>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  disabled={busy}
                  onClick={() => setRemoving(member)}
                >
                  Remove
                </Button>
              </StudioTd>
            </tr>
          ))}
          {members.length === 0 ? (
            <tr>
              <StudioTd colSpan={4}>Nobody yet.</StudioTd>
            </tr>
          ) : null}
        </tbody>
      </StudioTable>

      {invites.length ? (
        <div className="flex flex-col gap-2">
          <p className="text-[0.74rem] uppercase tracking-[0.08em] text-body-soft">
            Waiting on an answer
          </p>
          {invites.map((invite) => (
            <div
              key={invite.invite_id}
              data-invite={invite.invite_id}
              className="flex items-center justify-between gap-3 rounded-md border border-border px-3 py-2"
            >
              <p className="text-[0.8rem]">
                <span className="font-medium">{invite.target_user}</span>
                <span className="text-body-soft"> · {invite.role_preset}</span>
                {invite.note ? <span className="text-body-soft"> · {invite.note}</span> : null}
              </p>
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={busy}
                onClick={() => revoke(invite)}
              >
                Withdraw
              </Button>
            </div>
          ))}
        </div>
      ) : null}

      <ConfirmDialog
        open={!!removing}
        onClose={() => setRemoving(null)}
        onConfirm={async () => {
          const member = removing;
          setRemoving(null);
          if (member) await removeMember(member);
        }}
        title="Remove this member?"
        confirmLabel="Remove"
        cancelLabel="Cancel"
        variant="destructive"
        busy={busy}
      >
        <p className="text-[0.8rem] text-body-soft">
          <span className="font-mono">{removing?.user_id}</span> loses access to this project
          immediately. Anything they made stays; they can be invited again.
        </p>
      </ConfirmDialog>
    </SettingsSection>
  );
}
