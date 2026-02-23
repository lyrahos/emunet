import { useLocation, useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  Home,
  CircleDollarSign,
  HardDrive,
  User,
  Settings,
  MessageCircle,
  Users,
  ShoppingBag,
} from "lucide-react";

interface NavItem {
  icon: React.ElementType;
  label: string;
  path: string;
}

const mainNav: NavItem[] = [
  { icon: Home, label: "Home", path: "/" },
  { icon: CircleDollarSign, label: "Seeds", path: "/seeds" },
  { icon: HardDrive, label: "Earn", path: "/earn" },
  { icon: User, label: "You", path: "/you" },
];

const secondaryNav: NavItem[] = [
  { icon: Users, label: "Contacts", path: "/contacts" },
  { icon: MessageCircle, label: "Whisper", path: "/whisper" },
  { icon: ShoppingBag, label: "Purchases", path: "/purchases" },
];

export default function Sidebar() {
  const location = useLocation();
  const navigate = useNavigate();

  const isActive = (path: string) => {
    if (path === "/") return location.pathname === "/";
    return location.pathname.startsWith(path);
  };

  return (
    <aside className="flex flex-col h-full w-[240px] border-r border-[var(--color-border)] bg-[var(--color-sidebar)] select-none">
      {/* App Logo */}
      <div className="flex items-center gap-2.5 px-5 pt-5 pb-4">
        <div className="w-8 h-8 rounded-lg bg-[var(--color-accent)] flex items-center justify-center">
          <span className="text-white font-bold text-sm">O</span>
        </div>
        <span className="text-lg font-semibold text-[var(--color-text)]">
          Ochra
        </span>
      </div>

      {/* Main Navigation */}
      <nav className="flex-1 px-3 space-y-1">
        <p className="px-2 pt-3 pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--color-text-secondary)]">
          Main
        </p>
        {mainNav.map((item) => {
          const active = isActive(item.path);
          return (
            <button
              key={item.path}
              onClick={() => navigate(item.path)}
              className={`relative w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors
                ${
                  active
                    ? "text-[var(--color-accent)]"
                    : "text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30"
                }`}
            >
              {active && (
                <motion.div
                  layoutId="sidebar-active"
                  className="absolute inset-0 rounded-lg bg-[var(--color-accent)]/10"
                  transition={{ type: "spring", stiffness: 300, damping: 25 }}
                />
              )}
              <item.icon className="h-5 w-5 relative z-10" />
              <span className="relative z-10">{item.label}</span>
            </button>
          );
        })}

        <p className="px-2 pt-5 pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--color-text-secondary)]">
          Social
        </p>
        {secondaryNav.map((item) => {
          const active = isActive(item.path);
          return (
            <button
              key={item.path}
              onClick={() => navigate(item.path)}
              className={`relative w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors
                ${
                  active
                    ? "text-[var(--color-accent)]"
                    : "text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30"
                }`}
            >
              {active && (
                <motion.div
                  layoutId="sidebar-active-secondary"
                  className="absolute inset-0 rounded-lg bg-[var(--color-accent)]/10"
                  transition={{ type: "spring", stiffness: 300, damping: 25 }}
                />
              )}
              <item.icon className="h-5 w-5 relative z-10" />
              <span className="relative z-10">{item.label}</span>
            </button>
          );
        })}
      </nav>

      {/* Bottom: Settings + Version */}
      <div className="px-3 pb-4 space-y-1">
        <button
          onClick={() => navigate("/you")}
          className="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30 transition-colors"
        >
          <Settings className="h-5 w-5" />
          <span>Settings</span>
        </button>
        <p className="px-3 text-[10px] text-[var(--color-text-secondary)]/60">
          Ochra v0.1.0
        </p>
      </div>
    </aside>
  );
}
