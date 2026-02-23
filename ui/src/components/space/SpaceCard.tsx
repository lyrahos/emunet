import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { Users, Pin } from "lucide-react";
import Badge from "@/components/ui/Badge";
import { formatRelativeTime } from "@/lib/format";
import type { Space } from "@/lib/store";

interface SpaceCardProps {
  space: Space;
}

export default function SpaceCard({ space }: SpaceCardProps) {
  const navigate = useNavigate();

  return (
    <motion.button
      whileHover={{ y: -2 }}
      whileTap={{ scale: 0.98 }}
      transition={{ type: "spring", stiffness: 300, damping: 25 }}
      onClick={() => navigate(`/space/${space.groupId}`)}
      className="w-full text-left p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] hover:shadow-md transition-shadow"
    >
      <div className="flex items-start gap-3">
        {/* Space Icon */}
        <div className="w-11 h-11 rounded-xl bg-[var(--color-accent)]/10 flex items-center justify-center flex-shrink-0">
          {space.icon ? (
            <span className="text-xl">{space.icon}</span>
          ) : (
            <span className="text-lg font-semibold text-[var(--color-accent)]">
              {space.name.charAt(0).toUpperCase()}
            </span>
          )}
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-semibold text-[var(--color-text)] truncate">
              {space.name}
            </h3>
            {space.unread && (
              <span className="w-2 h-2 rounded-full bg-[var(--color-accent)] flex-shrink-0" />
            )}
            {space.pinned && (
              <Pin className="h-3 w-3 text-[var(--color-text-secondary)] flex-shrink-0" />
            )}
          </div>

          <div className="flex items-center gap-2 mt-1">
            <Badge variant={space.role} size="sm" />
            <span className="flex items-center gap-1 text-xs text-[var(--color-text-secondary)]">
              <Users className="h-3 w-3" />
              {space.memberCount}
            </span>
            <span className="text-xs text-[var(--color-text-secondary)]">
              {formatRelativeTime(space.lastActivity)}
            </span>
          </div>
        </div>
      </div>
    </motion.button>
  );
}
