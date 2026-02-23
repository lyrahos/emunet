import { Routes, Route } from "react-router-dom";
import Shell from "@/components/layout/Shell";
import Home from "@/pages/Home";
import Welcome from "@/pages/setup/Welcome";
import MeetSeeds from "@/pages/setup/MeetSeeds";
import EarnSetup from "@/pages/setup/EarnSetup";
import Recovery from "@/pages/setup/Recovery";
import Ready from "@/pages/setup/Ready";
import SpaceView from "@/pages/SpaceView";
import SpaceBuilder from "@/pages/SpaceBuilder";
import Seeds from "@/pages/Seeds";
import Earn from "@/pages/Earn";
import You from "@/pages/You";
import Contacts from "@/pages/Contacts";
import Whisper from "@/pages/Whisper";
import WhisperChat from "@/pages/WhisperChat";
import Dashboard from "@/pages/Dashboard";
import Checkout from "@/pages/Checkout";
import PurchaseLibrary from "@/pages/PurchaseLibrary";
import RecoverySetup from "@/pages/RecoverySetup";
import People from "@/pages/People";
import SpaceSettings from "@/pages/SpaceSettings";
import { useAppStore } from "@/lib/store";
import { ToastContainer } from "@/components/ui/Toast";

export default function App() {
  const theme = useAppStore((s) => s.theme);

  return (
    <div className={theme === "dark" ? "dark" : theme === "system" ? "" : ""}>
      <Routes>
        {/* Setup wizard (no shell) */}
        <Route path="/setup" element={<Welcome />} />
        <Route path="/setup/seeds" element={<MeetSeeds />} />
        <Route path="/setup/earn" element={<EarnSetup />} />
        <Route path="/setup/recovery" element={<Recovery />} />
        <Route path="/setup/ready" element={<Ready />} />

        {/* Main app routes inside Shell */}
        <Route element={<Shell />}>
          <Route path="/" element={<Home />} />
          <Route path="/space/new" element={<SpaceBuilder />} />
          <Route path="/space/:groupId" element={<SpaceView />} />
          <Route path="/seeds" element={<Seeds />} />
          <Route path="/earn" element={<Earn />} />
          <Route path="/you" element={<You />} />
          <Route path="/contacts" element={<Contacts />} />
          <Route path="/whisper" element={<Whisper />} />
          <Route path="/whisper/:sessionId" element={<WhisperChat />} />
          <Route path="/dashboard/:groupId" element={<Dashboard />} />
          <Route path="/checkout/:contentHash" element={<Checkout />} />
          <Route path="/purchases" element={<PurchaseLibrary />} />
          <Route path="/recovery" element={<RecoverySetup />} />
          <Route path="/space/:groupId/people" element={<People />} />
          <Route path="/space/:groupId/settings" element={<SpaceSettings />} />
        </Route>
      </Routes>
      <ToastContainer />
    </div>
  );
}
