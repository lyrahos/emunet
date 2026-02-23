import { useState, useEffect } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  ArrowLeft,
  Users,
  Sprout,
  TrendingUp,
  Eye,
  Clock,
} from "lucide-react";
import Card from "@/components/ui/Card";
import { rpcCall } from "@/lib/rpc-client";
import { formatSeeds, formatRelativeTime } from "@/lib/format";

interface DashboardStats {
  totalMembers: number;
  totalContent: number;
  totalEarnings: number; // micro-seeds
  viewsToday: number;
  earningsThisWeek: number;
}

interface ActivityItem {
  id: string;
  type: "join" | "purchase" | "publish" | "leave";
  actorName: string;
  description: string;
  timestamp: number;
  amount?: number; // micro-seeds
}

export default function Dashboard() {
  const { groupId } = useParams<{ groupId: string }>();
  const navigate = useNavigate();

  const [stats, setStats] = useState<DashboardStats>({
    totalMembers: 0,
    totalContent: 0,
    totalEarnings: 0,
    viewsToday: 0,
    earningsThisWeek: 0,
  });
  const [activity, setActivity] = useState<ActivityItem[]>([]);

  useEffect(() => {
    if (!groupId) return;
    rpcCall<DashboardStats>("get_group_stats", { group_id: groupId })
      .then(setStats)
      .catch(() => {});
    rpcCall<ActivityItem[]>("get_group_activity", { group_id: groupId })
      .then(setActivity)
      .catch(() => {});
  }, [groupId]);

  const statCards = [
    {
      icon: Users,
      label: "Members",
      value: stats.totalMembers.toString(),
      color: "text-blue-500",
      bg: "bg-blue-100 dark:bg-blue-900/30",
    },
    {
      icon: Eye,
      label: "Views Today",
      value: stats.viewsToday.toString(),
      color: "text-green-500",
      bg: "bg-green-100 dark:bg-green-900/30",
    },
    {
      icon: Sprout,
      label: "Total Earnings",
      value: formatSeeds(stats.totalEarnings),
      color: "text-[var(--color-accent)]",
      bg: "bg-[var(--color-accent)]/10",
    },
    {
      icon: TrendingUp,
      label: "This Week",
      value: formatSeeds(stats.earningsThisWeek),
      color: "text-purple-500",
      bg: "bg-purple-100 dark:bg-purple-900/30",
    },
  ];

  const activityIcons: Record<string, string> = {
    join: "text-green-500",
    purchase: "text-[var(--color-accent)]",
    publish: "text-blue-500",
    leave: "text-red-500",
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-3xl mx-auto px-6 py-8 space-y-8">
        {/* Header */}
        <div className="flex items-center gap-3">
          <button
            onClick={() => navigate(`/space/${groupId}`)}
            className="p-1.5 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30 transition-colors"
          >
            <ArrowLeft className="h-5 w-5" />
          </button>
          <div>
            <h1 className="text-xl font-bold text-[var(--color-text)]">
              Host Dashboard
            </h1>
            <p className="text-sm text-[var(--color-text-secondary)]">
              Overview of your Space performance
            </p>
          </div>
        </div>

        {/* Stats grid */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
          {statCards.map((stat, i) => (
            <motion.div
              key={stat.label}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.08 }}
            >
              <Card padding="md">
                <div className={`w-9 h-9 rounded-xl ${stat.bg} flex items-center justify-center mb-3`}>
                  <stat.icon className={`h-4 w-4 ${stat.color}`} />
                </div>
                <p className="text-xl font-bold text-[var(--color-text)]">
                  {stat.value}
                </p>
                <p className="text-xs text-[var(--color-text-secondary)] mt-0.5">
                  {stat.label}
                </p>
              </Card>
            </motion.div>
          ))}
        </div>

        {/* Earnings detail */}
        <Card padding="lg">
          <h2 className="text-base font-semibold text-[var(--color-text)] mb-4">
            Earnings Breakdown
          </h2>
          <div className="space-y-3">
            <div className="flex items-center justify-between py-2 border-b border-[var(--color-border)]">
              <span className="text-sm text-[var(--color-text-secondary)]">
                Content Sales
              </span>
              <span className="text-sm font-semibold text-[var(--color-text)]">
                {formatSeeds(stats.totalEarnings)}
              </span>
            </div>
            <div className="flex items-center justify-between py-2 border-b border-[var(--color-border)]">
              <span className="text-sm text-[var(--color-text-secondary)]">
                Host Commission
              </span>
              <span className="text-sm font-semibold text-[var(--color-text)]">
                {formatSeeds(Math.floor(stats.totalEarnings * 0.05))}
              </span>
            </div>
            <div className="flex items-center justify-between py-2">
              <span className="text-sm text-[var(--color-text-secondary)]">
                Network Fees
              </span>
              <span className="text-sm font-semibold text-[var(--color-text)]">
                {formatSeeds(Math.floor(stats.totalEarnings * 0.01))}
              </span>
            </div>
          </div>
        </Card>

        {/* Activity feed */}
        <div>
          <h2 className="text-base font-semibold text-[var(--color-text)] mb-4">
            Recent Activity
          </h2>
          {activity.length > 0 ? (
            <div className="space-y-2">
              {activity.map((item, i) => (
                <motion.div
                  key={item.id}
                  initial={{ opacity: 0, x: -10 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: i * 0.03 }}
                  className="flex items-center gap-3 p-3 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]"
                >
                  <div
                    className={`w-2 h-2 rounded-full ${activityIcons[item.type] ?? "text-gray-500"}`}
                    style={{
                      backgroundColor: "currentColor",
                    }}
                  />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm text-[var(--color-text)]">
                      <span className="font-semibold">{item.actorName}</span>{" "}
                      {item.description}
                    </p>
                  </div>
                  {item.amount && (
                    <span className="text-xs font-semibold text-[var(--color-accent)] flex-shrink-0">
                      +{formatSeeds(item.amount)}
                    </span>
                  )}
                  <span className="text-xs text-[var(--color-text-secondary)] flex-shrink-0">
                    {formatRelativeTime(item.timestamp)}
                  </span>
                </motion.div>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center py-12 text-center">
              <Clock className="h-8 w-8 text-[var(--color-text-secondary)]/30 mb-3" />
              <p className="text-sm text-[var(--color-text-secondary)]">
                No recent activity
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
