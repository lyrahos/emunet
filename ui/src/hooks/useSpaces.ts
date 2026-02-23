import { useEffect, useCallback } from "react";
import { useAppStore, type Space } from "@/lib/store";
import { rpcCall } from "@/lib/rpc-client";
import { useEvents } from "./useEvents";

interface GroupResponse {
  group_id: string;
  name: string;
  icon?: string;
  role: "host" | "creator" | "moderator" | "member";
  member_count: number;
  last_activity: number;
  unread: boolean;
  pinned: boolean;
  template: "storefront" | "forum" | "newsfeed" | "gallery" | "library";
}

function mapGroup(g: GroupResponse): Space {
  return {
    groupId: g.group_id,
    name: g.name,
    icon: g.icon,
    role: g.role,
    memberCount: g.member_count,
    lastActivity: g.last_activity,
    unread: g.unread,
    pinned: g.pinned,
    template: g.template,
  };
}

/**
 * Hook that fetches and keeps the Space list in sync.
 * Listens for group_updated events from the daemon.
 */
export function useSpaces() {
  const spaces = useAppStore((s) => s.spaces);
  const setSpaces = useAppStore((s) => s.setSpaces);
  const unlocked = useAppStore((s) => s.auth.unlocked);

  const fetchSpaces = useCallback(async () => {
    try {
      const res = await rpcCall<GroupResponse[]>("get_my_groups");
      setSpaces(res.map(mapGroup));
    } catch {
      // Groups may not be available yet
    }
  }, [setSpaces]);

  // Initial fetch
  useEffect(() => {
    if (unlocked) {
      fetchSpaces();
    }
  }, [unlocked, fetchSpaces]);

  // Listen for changes
  useEvents(
    "group_updated",
    () => {
      fetchSpaces();
    },
    unlocked,
  );

  // Sort: pinned first, then by last activity
  const sortedSpaces = [...spaces].sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
    return b.lastActivity - a.lastActivity;
  });

  return {
    spaces: sortedSpaces,
    refetch: fetchSpaces,
  };
}
