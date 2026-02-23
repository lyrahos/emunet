import { useState, useEffect } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  ArrowLeft,
  UserPlus,
  MoreVertical,
  ShieldCheck,
  ShieldOff,
  Crown,
} from "lucide-react";
import Button from "@/components/ui/Button";
import Badge from "@/components/ui/Badge";
import Modal from "@/components/ui/Modal";
import { rpcCall } from "@/lib/rpc-client";
import { useAppStore } from "@/lib/store";

type Role = "host" | "creator" | "moderator" | "member";

interface Member {
  id: string;
  displayName: string;
  role: Role;
  joinedAt: number;
  online: boolean;
}

export default function People() {
  const { groupId } = useParams<{ groupId: string }>();
  const navigate = useNavigate();
  const addToast = useAppStore((s) => s.addToast);

  const [members, setMembers] = useState<Member[]>([]);
  const [inviteOpen, setInviteOpen] = useState(false);
  const [actionMember, setActionMember] = useState<Member | null>(null);

  useEffect(() => {
    if (!groupId) return;
    rpcCall<Member[]>("get_group_members", { group_id: groupId })
      .then(setMembers)
      .catch(() => {});
  }, [groupId]);

  const handlePromote = async (memberId: string, newRole: Role) => {
    try {
      await rpcCall("set_member_role", {
        group_id: groupId,
        member_id: memberId,
        role: newRole,
      });
      setMembers((prev) =>
        prev.map((m) => (m.id === memberId ? { ...m, role: newRole } : m)),
      );
      setActionMember(null);
      addToast({ type: "success", message: `Role updated.` });
    } catch {
      addToast({ type: "error", message: "Failed to update role." });
    }
  };

  const rolePriority: Record<Role, number> = {
    host: 0,
    creator: 1,
    moderator: 2,
    member: 3,
  };

  const sortedMembers = [...members].sort(
    (a, b) => rolePriority[a.role] - rolePriority[b.role],
  );

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-2xl mx-auto px-6 py-8 space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <button
              onClick={() => navigate(`/space/${groupId}`)}
              className="p-1.5 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30 transition-colors"
            >
              <ArrowLeft className="h-5 w-5" />
            </button>
            <div>
              <h1 className="text-xl font-bold text-[var(--color-text)]">
                Members
              </h1>
              <p className="text-sm text-[var(--color-text-secondary)]">
                {members.length} members
              </p>
            </div>
          </div>
          <Button size="sm" onClick={() => setInviteOpen(true)}>
            <UserPlus className="h-4 w-4" />
            Invite
          </Button>
        </div>

        {/* Member list */}
        <div className="space-y-2">
          {sortedMembers.map((member, i) => (
            <motion.div
              key={member.id}
              initial={{ opacity: 0, y: 5 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.03 }}
              className="flex items-center gap-3 p-3 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]"
            >
              <div className="relative">
                <div className="w-10 h-10 rounded-full bg-[var(--color-accent)]/10 flex items-center justify-center">
                  <span className="text-sm font-semibold text-[var(--color-accent)]">
                    {member.displayName.charAt(0).toUpperCase()}
                  </span>
                </div>
                {member.online && (
                  <div className="absolute bottom-0 right-0 w-3 h-3 rounded-full bg-green-500 border-2 border-[var(--color-surface)]" />
                )}
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <p className="text-sm font-semibold text-[var(--color-text)] truncate">
                    {member.displayName}
                  </p>
                  <Badge variant={member.role} size="sm" />
                </div>
              </div>
              {member.role !== "host" && (
                <button
                  onClick={() => setActionMember(member)}
                  className="p-1.5 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30 transition-colors"
                >
                  <MoreVertical className="h-4 w-4" />
                </button>
              )}
            </motion.div>
          ))}
        </div>
      </div>

      {/* Member Actions Modal */}
      <Modal
        open={!!actionMember}
        onClose={() => setActionMember(null)}
        title={actionMember?.displayName ?? ""}
      >
        {actionMember && (
          <div className="space-y-2">
            <p className="text-sm text-[var(--color-text-secondary)] mb-4">
              Current role: <Badge variant={actionMember.role} size="sm" />
            </p>

            {actionMember.role !== "creator" && (
              <button
                onClick={() => handlePromote(actionMember.id, "creator")}
                className="w-full flex items-center gap-3 px-4 py-3 rounded-xl hover:bg-[var(--color-border)]/20 transition-colors text-left"
              >
                <ShieldCheck className="h-5 w-5 text-blue-500" />
                <div>
                  <p className="text-sm font-medium text-[var(--color-text)]">
                    Promote to Creator
                  </p>
                  <p className="text-xs text-[var(--color-text-secondary)]">
                    Can publish and sell content
                  </p>
                </div>
              </button>
            )}

            {actionMember.role !== "moderator" && (
              <button
                onClick={() => handlePromote(actionMember.id, "moderator")}
                className="w-full flex items-center gap-3 px-4 py-3 rounded-xl hover:bg-[var(--color-border)]/20 transition-colors text-left"
              >
                <Crown className="h-5 w-5 text-purple-500" />
                <div>
                  <p className="text-sm font-medium text-[var(--color-text)]">
                    Promote to Moderator
                  </p>
                  <p className="text-xs text-[var(--color-text-secondary)]">
                    Can manage members and content
                  </p>
                </div>
              </button>
            )}

            {actionMember.role !== "member" && (
              <button
                onClick={() => handlePromote(actionMember.id, "member")}
                className="w-full flex items-center gap-3 px-4 py-3 rounded-xl hover:bg-[var(--color-border)]/20 transition-colors text-left"
              >
                <ShieldOff className="h-5 w-5 text-gray-500" />
                <div>
                  <p className="text-sm font-medium text-[var(--color-text)]">
                    Demote to Member
                  </p>
                  <p className="text-xs text-[var(--color-text-secondary)]">
                    Standard member permissions
                  </p>
                </div>
              </button>
            )}
          </div>
        )}
      </Modal>

      {/* Invite Modal */}
      <Modal
        open={inviteOpen}
        onClose={() => setInviteOpen(false)}
        title="Invite to Space"
      >
        <div className="space-y-4">
          <p className="text-sm text-[var(--color-text-secondary)]">
            Share this invite link with people you want to add to this Space.
          </p>
          <div className="p-4 rounded-xl bg-[var(--color-border)]/20 text-center">
            <p className="text-xs text-[var(--color-text-secondary)] break-all font-mono">
              ochra://invite/{groupId ?? "..."}
            </p>
          </div>
          <Button
            variant="secondary"
            className="w-full"
            onClick={() => {
              navigator.clipboard.writeText(`ochra://invite/${groupId}`);
              addToast({ type: "info", message: "Invite link copied!" });
            }}
          >
            Copy Invite Link
          </Button>
        </div>
      </Modal>
    </div>
  );
}
