"use client";

import { useState } from "react";

// ── Tooltip data ──────────────────────────────────────────────────────────────

interface ProofPhase {
  key: string;
  label: string;
  what: string;
  why: string;
  verified: string;
}

const PROOF_PHASES: ProofPhase[] = [
  {
    key: "deal",
    label: "DEAL PROOF",
    what: 'This proof shows that the deck was shuffled fairly and each player received exactly 2 cards from it — without anyone knowing the other cards.',
    why: "No one — not even the coordinator — can see your hole cards. The shuffle happened across 3 independent MPC nodes that never share their secrets.",
    verified: '"Verified on-chain" means the Soroban contract checked the cryptographic proof and confirmed the deck commitment matches what was posted before the hand.',
  },
  {
    key: "flop",
    label: "FLOP PROOF",
    what: "This proof shows that the 3 community cards revealed on the flop were already committed to in the original deck, and are now being revealed honestly.",
    why: "The dealer cannot choose which cards to show after seeing your hand — the commitment was locked on-chain before any cards were revealed.",
    verified: '"Verified on-chain" means the contract confirmed these 3 cards are consistent with the deck root posted at the start of the hand.',
  },
  {
    key: "turn",
    label: "TURN PROOF",
    what: "This proof shows that the 4th community card (the turn) matches the original committed deck.",
    why: "The same zero-knowledge guarantee as the flop — the card cannot be swapped or changed after the deck was committed.",
    verified: '"Verified on-chain" means the Soroban contract validated the turn card against the original deck root.',
  },
  {
    key: "river",
    label: "RIVER PROOF",
    what: "This proof shows that the 5th and final community card (the river) matches the original committed deck.",
    why: "Ensures the river card is deterministic from the shuffle, not chosen by any party after seeing the board.",
    verified: '"Verified on-chain" means the river card is cryptographically bound to the deck committed at hand start.',
  },
  {
    key: "showdown",
    label: "SHOWDOWN PROOF",
    what: "This proof shows that each player's hole cards are the ones that were secretly assigned to them in the deal proof, and that the declared hand rank is computed correctly.",
    why: "Players cannot lie about their cards at showdown. The ZK proof forces an honest reveal without exposing cards that were folded.",
    verified: '"Verified on-chain" means the Soroban contract confirmed the winning hand rank and awarded the pot correctly — no trusted arbiter needed.',
  },
];

// ── Tooltip component ─────────────────────────────────────────────────────────

function InfoTooltip({ text }: { text: string }) {
  const [open, setOpen] = useState(false);
  return (
    <span style={{ position: "relative", display: "inline-block", marginLeft: 4 }}>
      <button
        aria-label="More information"
        onClick={() => setOpen((v) => !v)}
        style={{
          background: "none",
          border: "1px solid #7f8c8d",
          borderRadius: "50%",
          color: "#95a5a6",
          cursor: "pointer",
          fontSize: "7px",
          lineHeight: 1,
          padding: "1px 3px",
          verticalAlign: "middle",
        }}
      >
        i
      </button>
      {open && (
        <>
          {/* Backdrop to close on outside click */}
          <span
            style={{ position: "fixed", inset: 0, zIndex: 10 }}
            onClick={() => setOpen(false)}
          />
          <span
            style={{
              position: "absolute",
              bottom: "calc(100% + 6px)",
              left: "50%",
              transform: "translateX(-50%)",
              zIndex: 20,
              background: "rgba(12, 10, 24, 0.97)",
              border: "1px solid #c47d2e",
              borderRadius: 4,
              padding: "8px 10px",
              width: 220,
              fontSize: "8px",
              color: "#f5e6c8",
              lineHeight: 1.6,
              pointerEvents: "auto",
            }}
          >
            {text}
          </span>
        </>
      )}
    </span>
  );
}

// ── Phase row ─────────────────────────────────────────────────────────────────

interface PhaseRowProps {
  phase: ProofPhase;
  status?: "pending" | "verified" | "missing";
  txHash?: string;
  proofSize?: number;
}

function PhaseRow({ phase, status = "pending", txHash, proofSize }: PhaseRowProps) {
  const statusColor =
    status === "verified" ? "#27ae60" : status === "missing" ? "#e74c3c" : "#7f8c8d";
  const statusLabel =
    status === "verified" ? "✔ VERIFIED" : status === "missing" ? "✗ MISSING" : "— PENDING";

  return (
    <div
      style={{
        borderBottom: "1px solid rgba(196,125,46,0.2)",
        paddingBottom: 8,
        marginBottom: 8,
      }}
    >
      <div className="flex items-center justify-between">
        <span style={{ fontSize: "9px", color: "#f5e6c8", fontWeight: "bold" }}>
          {phase.label}
          <InfoTooltip
            text={`WHAT: ${phase.what}\n\nWHY IT MATTERS: ${phase.why}`}
          />
        </span>
        <span style={{ fontSize: "8px", color: statusColor }}>
          {statusLabel}
          {status === "verified" && (
            <InfoTooltip text={phase.verified} />
          )}
        </span>
      </div>
      {proofSize !== undefined && (
        <div style={{ fontSize: "7px", color: "#7f8c8d", marginTop: 2 }}>
          PROOF SIZE: {proofSize.toLocaleString()} bytes
        </div>
      )}
      {txHash && (
        <div
          style={{
            fontSize: "7px",
            color: "#3498db",
            marginTop: 2,
            wordBreak: "break-all",
          }}
        >
          TX: {txHash.slice(0, 12)}…{txHash.slice(-8)}
        </div>
      )}
    </div>
  );
}

// ── Main component ────────────────────────────────────────────────────────────

export interface ProofExplorerData {
  dealTxHash?: string;
  revealTxHashes?: Record<string, string>;
  showdownTxHash?: string;
}

interface ProofExplorerProps {
  data?: ProofExplorerData;
}

export function ProofExplorer({ data }: ProofExplorerProps) {
  const [howItWorksOpen, setHowItWorksOpen] = useState(false);

  const txFor = (key: string): string | undefined => {
    if (key === "deal") return data?.dealTxHash;
    if (key === "showdown") return data?.showdownTxHash;
    return data?.revealTxHashes?.[key];
  };

  const statusFor = (key: string): "pending" | "verified" | "missing" => {
    const tx = txFor(key);
    return tx ? "verified" : "pending";
  };

  return (
    <div style={{ padding: "12px 0" }}>
      {/* How it works collapsible */}
      <button
        onClick={() => setHowItWorksOpen((v) => !v)}
        style={{
          background: "none",
          border: "1px solid #c47d2e",
          borderRadius: 3,
          color: "#c47d2e",
          cursor: "pointer",
          fontSize: "8px",
          padding: "4px 8px",
          marginBottom: 10,
          width: "100%",
          textAlign: "left",
        }}
      >
        {howItWorksOpen ? "▼" : "▶"} HOW ZERO-KNOWLEDGE PROOFS WORK
      </button>

      {howItWorksOpen && (
        <div
          style={{
            background: "rgba(30,20,10,0.6)",
            border: "1px solid rgba(196,125,46,0.3)",
            borderRadius: 4,
            fontSize: "8px",
            color: "#95a5a6",
            lineHeight: 1.7,
            marginBottom: 12,
            padding: "10px 12px",
          }}
        >
          <p style={{ marginBottom: 6, color: "#f5e6c8" }}>
            StellPoker uses zero-knowledge (ZK) proofs to guarantee fairness
            without a trusted dealer.
          </p>
          <p style={{ marginBottom: 6 }}>
            <strong style={{ color: "#c47d2e" }}>Commit:</strong> Before cards are
            revealed, a cryptographic commitment to the full shuffled deck is posted
            on the Soroban blockchain. Nobody can change the deck after this point.
          </p>
          <p style={{ marginBottom: 6 }}>
            <strong style={{ color: "#c47d2e" }}>Prove:</strong> Each reveal (deal,
            flop, turn, river, showdown) comes with a ZK proof that the revealed cards
            match the original commitment — without revealing any other cards.
          </p>
          <p>
            <strong style={{ color: "#c47d2e" }}>Verify:</strong> The Soroban smart
            contract checks each proof on-chain. No one — not even the server — can
            cheat undetected.
          </p>
        </div>
      )}

      {/* Phase list */}
      {PROOF_PHASES.map((phase) => (
        <PhaseRow
          key={phase.key}
          phase={phase}
          status={statusFor(phase.key)}
          txHash={txFor(phase.key)}
        />
      ))}

      <div style={{ fontSize: "7px", color: "#4a4a6a", marginTop: 8 }}>
        Proofs use UltraHonk / Barretenberg via coNoir MPC.
      </div>
    </div>
  );
}
