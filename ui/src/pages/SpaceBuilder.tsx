import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { motion, AnimatePresence } from "framer-motion";
import {
  ArrowLeft,
  ArrowRight,
  Store,
  MessageSquare,
  Newspaper,
  Image,
  Library,
  UserPlus,
  Check,
  X,
} from "lucide-react";
import Button from "@/components/ui/Button";
import { useIpc } from "@/hooks/useIpc";
import { useAppStore } from "@/lib/store";

type Template = "storefront" | "forum" | "newsfeed" | "gallery" | "library";

interface TemplateOption {
  id: Template;
  label: string;
  description: string;
  icon: React.ElementType;
}

const templates: TemplateOption[] = [
  {
    id: "storefront",
    label: "Storefront",
    description: "Grid of purchasable content cards with prices",
    icon: Store,
  },
  {
    id: "forum",
    label: "Forum",
    description: "Threaded discussions and conversations",
    icon: MessageSquare,
  },
  {
    id: "newsfeed",
    label: "News Feed",
    description: "Vertical feed of posts and updates",
    icon: Newspaper,
  },
  {
    id: "gallery",
    label: "Gallery",
    description: "Visual mosaic of images and media",
    icon: Image,
  },
  {
    id: "library",
    label: "Library",
    description: "Sortable list of files and documents",
    icon: Library,
  },
];

export default function SpaceBuilder() {
  const navigate = useNavigate();
  const { call, loading } = useIpc();
  const addToast = useAppStore((s) => s.addToast);

  const [step, setStep] = useState(0);
  const [name, setName] = useState("");
  const [template, setTemplate] = useState<Template>("storefront");
  const [invites, setInvites] = useState<string[]>([]);
  const [inviteInput, setInviteInput] = useState("");

  const steps = ["Name", "Style", "Invite", "Summary"];

  const addInvite = () => {
    const trimmed = inviteInput.trim();
    if (trimmed && !invites.includes(trimmed)) {
      setInvites((prev) => [...prev, trimmed]);
      setInviteInput("");
    }
  };

  const removeInvite = (invite: string) => {
    setInvites((prev) => prev.filter((i) => i !== invite));
  };

  const handleCreate = async () => {
    const result = await call("create_group", {
      name: name.trim(),
      template,
      invite_codes: invites,
    });

    if (result) {
      const groupId = (result as { group_id: string }).group_id;
      addToast({ type: "success", message: `Space "${name}" created!` });
      navigate(`/space/${groupId}`);
    } else {
      addToast({ type: "error", message: "Failed to create Space." });
    }
  };

  return (
    <div className="h-full flex flex-col">
      <header className="flex items-center gap-4 px-6 pt-6 pb-4">
        <button
          onClick={() => (step > 0 ? setStep(step - 1) : navigate(-1))}
          className="p-1.5 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30 transition-colors"
        >
          <ArrowLeft className="h-5 w-5" />
        </button>
        <div>
          <h1 className="text-xl font-bold text-[var(--color-text)]">
            New Space
          </h1>
          <p className="text-sm text-[var(--color-text-secondary)]">
            Step {step + 1} of {steps.length}: {steps[step]}
          </p>
        </div>
      </header>

      {/* Step progress */}
      <div className="px-6 pb-6">
        <div className="flex gap-1.5">
          {steps.map((_, i) => (
            <div
              key={i}
              className={`h-1 flex-1 rounded-full transition-colors ${
                i <= step ? "bg-[var(--color-accent)]" : "bg-[var(--color-border)]"
              }`}
            />
          ))}
        </div>
      </div>

      {/* Step content */}
      <div className="flex-1 overflow-y-auto px-6 pb-6">
        <AnimatePresence mode="wait">
          {step === 0 && (
            <motion.div
              key="name"
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              className="space-y-4"
            >
              <label className="block text-sm font-medium text-[var(--color-text)]">
                Space Name
              </label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Give your Space a name"
                maxLength={48}
                className="w-full px-4 py-3 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50"
              />
              <p className="text-xs text-[var(--color-text-secondary)]">
                This is what members will see. You can change it later.
              </p>
              <Button
                className="w-full"
                disabled={name.trim().length < 2}
                onClick={() => setStep(1)}
              >
                Next
                <ArrowRight className="h-4 w-4" />
              </Button>
            </motion.div>
          )}

          {step === 1 && (
            <motion.div
              key="style"
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              className="space-y-4"
            >
              <p className="text-sm text-[var(--color-text-secondary)]">
                Choose a layout template for your Space.
              </p>
              <div className="grid gap-3">
                {templates.map((t) => {
                  const active = template === t.id;
                  return (
                    <button
                      key={t.id}
                      onClick={() => setTemplate(t.id)}
                      className={`flex items-center gap-4 p-4 rounded-xl border text-left transition-colors ${
                        active
                          ? "border-[var(--color-accent)] bg-[var(--color-accent)]/5"
                          : "border-[var(--color-border)] bg-[var(--color-surface)] hover:border-[var(--color-accent)]/40"
                      }`}
                    >
                      <div
                        className={`w-10 h-10 rounded-xl flex items-center justify-center ${
                          active
                            ? "bg-[var(--color-accent)]/10"
                            : "bg-[var(--color-border)]/30"
                        }`}
                      >
                        <t.icon
                          className={`h-5 w-5 ${
                            active
                              ? "text-[var(--color-accent)]"
                              : "text-[var(--color-text-secondary)]"
                          }`}
                        />
                      </div>
                      <div>
                        <p
                          className={`text-sm font-semibold ${
                            active
                              ? "text-[var(--color-accent)]"
                              : "text-[var(--color-text)]"
                          }`}
                        >
                          {t.label}
                        </p>
                        <p className="text-xs text-[var(--color-text-secondary)]">
                          {t.description}
                        </p>
                      </div>
                      {active && (
                        <Check className="h-5 w-5 text-[var(--color-accent)] ml-auto" />
                      )}
                    </button>
                  );
                })}
              </div>
              <Button className="w-full" onClick={() => setStep(2)}>
                Next
                <ArrowRight className="h-4 w-4" />
              </Button>
            </motion.div>
          )}

          {step === 2 && (
            <motion.div
              key="invite"
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              className="space-y-4"
            >
              <p className="text-sm text-[var(--color-text-secondary)]">
                Invite creators to your Space. You can also do this later.
              </p>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={inviteInput}
                  onChange={(e) => setInviteInput(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && addInvite()}
                  placeholder="Paste contact ID or invite code"
                  className="flex-1 px-4 py-2.5 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50 text-sm"
                />
                <Button variant="secondary" onClick={addInvite}>
                  <UserPlus className="h-4 w-4" />
                </Button>
              </div>
              {invites.length > 0 && (
                <div className="space-y-2">
                  {invites.map((invite) => (
                    <div
                      key={invite}
                      className="flex items-center gap-2 px-3 py-2 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)]"
                    >
                      <span className="flex-1 text-sm text-[var(--color-text)] truncate">
                        {invite}
                      </span>
                      <button
                        onClick={() => removeInvite(invite)}
                        className="text-[var(--color-text-secondary)] hover:text-red-500 transition-colors"
                      >
                        <X className="h-4 w-4" />
                      </button>
                    </div>
                  ))}
                </div>
              )}
              <div className="flex gap-3">
                <Button className="flex-1" onClick={() => setStep(3)}>
                  Next
                  <ArrowRight className="h-4 w-4" />
                </Button>
                {invites.length === 0 && (
                  <Button
                    variant="ghost"
                    className="flex-1"
                    onClick={() => setStep(3)}
                  >
                    Skip
                  </Button>
                )}
              </div>
            </motion.div>
          )}

          {step === 3 && (
            <motion.div
              key="summary"
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              className="space-y-4"
            >
              <p className="text-sm text-[var(--color-text-secondary)]">
                Review your new Space before creating it.
              </p>
              <div className="space-y-3">
                <div className="p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]">
                  <p className="text-xs text-[var(--color-text-secondary)]">
                    Name
                  </p>
                  <p className="text-sm font-semibold text-[var(--color-text)]">
                    {name}
                  </p>
                </div>
                <div className="p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]">
                  <p className="text-xs text-[var(--color-text-secondary)]">
                    Layout
                  </p>
                  <p className="text-sm font-semibold text-[var(--color-text)] capitalize">
                    {template}
                  </p>
                </div>
                <div className="p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]">
                  <p className="text-xs text-[var(--color-text-secondary)]">
                    Invited Creators
                  </p>
                  <p className="text-sm font-semibold text-[var(--color-text)]">
                    {invites.length === 0 ? "None" : `${invites.length} people`}
                  </p>
                </div>
              </div>
              <Button
                className="w-full"
                size="lg"
                loading={loading}
                onClick={handleCreate}
              >
                Create Space
              </Button>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
