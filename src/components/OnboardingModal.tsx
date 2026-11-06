import React from "react";

/** First-run guide that explains how the deck is verified. */
export const OnboardingModal: React.FC<{ onClose: () => void }> = ({ onClose }) => {
  return (    <div className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4">
      <div className="w-full max-w-lg rounded-xl bg-[#151925] border border-[#394158] p-6 flex flex-col gap-4 shadow-2xl">
        <div className="flex items-center justify-between">
          <h2 className="text-base font-bold">
            DeckForge — Universal Stream Deck Verification Guide
          </h2>
          <button
            onClick={() => onClose()}
            className="text-xs text-[#A6AFC4] hover:text-white cursor-pointer"
          >
            ✕
          </button>
        </div>
        <div className="text-xs text-[#A6AFC4] space-y-2 leading-relaxed">
          <p>
            <strong className="text-white">1. Verified Hardware Detected:</strong>{" "}
            <code>Ajazz AKP05E Pro (USB\VID_0300&amp;PID_3013)</code> with 10 LCD Keys,
            Secondary Touch Strip, and 4 Rotary Encoders.
          </p>
          <p>
            <strong className="text-white">2. Zero-Freeze Guarantee:</strong> Unlike the
            official app (which re-sends all JPEG keys every 1–3 seconds), DeckForge hashes
            every frame with SHA-256 in Rust and only writes changed keys over USB.
          </p>
          <p>
            <strong className="text-white">3. Saved vs. Applied:</strong> Edits save locally
            first; click <strong>Apply to Device</strong> to push changed keys to hardware.
          </p>
        </div>
        <div className="flex justify-end">
          <button
            onClick={() => onClose()}
            className="h-9 px-4 rounded-md bg-[#8B5CF6] hover:bg-[#9D75F8] text-xs font-semibold text-white cursor-pointer"
          >
            Got It
          </button>
        </div>
      </div>
    </div>
  );
};
