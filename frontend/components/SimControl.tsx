"use client";

import React, { useState } from "react";
import { 
  ShieldAlert, 
  BatteryWarning, 
  SunDim, 
  Orbit, 
  RadioTower, 
  Flame, 
  CirclePlay,
  RotateCcw
} from "lucide-react";

interface SimControlProps {
  onInjectFailure: (failureType: string) => void;
  onResetSimulator: () => void;
  isSimulating: boolean;
}

const SimControl = React.memo(function SimControl({ 
  onInjectFailure, 
  onResetSimulator, 
  isSimulating 
}: SimControlProps) {
  const [activeFailure, setActiveFailure] = useState<string | null>(null);

  const failures = [
    {
      id: "battery_drain",
      label: "Battery Rapid Drain",
      desc: "Simulate solar battery depletion / high load cell fault.",
      icon: BatteryWarning,
      color: "border-rose-500/30 hover:border-rose-500 hover:bg-rose-500/5 text-rose-400"
    },
    {
      id: "solar_degrade",
      label: "Solar Panel Fail",
      desc: "Inject partial solar wing blocking or alignment jam.",
      icon: SunDim,
      color: "border-amber-500/30 hover:border-amber-500 hover:bg-amber-500/5 text-amber-400"
    },
    {
      id: "orbit_decay",
      label: "Altitude Orbit Decay",
      desc: "Trigger high-altitude atmospheric drag friction decay.",
      icon: Orbit,
      color: "border-cyan-500/30 hover:border-cyan-500 hover:bg-cyan-500/5 text-cyan-400"
    },
    {
      id: "signal_loss",
      label: "Comms Link Drop",
      desc: "Simulate radio noise anomaly or satellite antenna lag.",
      icon: RadioTower,
      color: "border-violet-500/30 hover:border-violet-500 hover:bg-violet-500/5 text-violet-400"
    },
    {
      id: "thruster_overheat",
      label: "Thruster Thermal Runaway",
      desc: "Inject high-pressure hydrazine thruster valve leak.",
      icon: Flame,
      color: "border-orange-500/30 hover:border-orange-500 hover:bg-orange-500/5 text-orange-400"
    }
  ];

  const handleInject = (id: string) => {
    setActiveFailure(id);
    onInjectFailure(id);
    // clear highlight after a short duration
    setTimeout(() => {
      setActiveFailure(null);
    }, 2000);
  };

  return (
    <div className="glass-panel border-[#27272a]/50 rounded-xl p-5 flex flex-col h-full">
      {/* Title */}
      <div className="border-b border-[#27272a]/30 pb-3 mb-4 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <ShieldAlert className="h-4.5 w-4.5 text-rose-500" />
          <h3 className="text-sm font-bold text-white font-mono uppercase tracking-wider">Failure Injection Deck</h3>
        </div>
        
        <div className="flex items-center gap-1.5 text-[10px] font-mono">
          <span className="text-zinc-500">SIM STATUS:</span>
          <span className={`flex items-center gap-1 font-semibold ${isSimulating ? "text-emerald-400" : "text-zinc-500"}`}>
            <span className={`h-1.5 w-1.5 rounded-full ${isSimulating ? "bg-emerald-400 pulse-green" : "bg-zinc-600"}`} />
            {isSimulating ? "RUNNING" : "STANDBY"}
          </span>
        </div>
      </div>

      <p className="text-[11px] text-zinc-400 font-mono mb-4">
        Select a hardware system failure scenario to override telemetry inputs and trigger alerts.
      </p>

      {/* Failures Grid */}
      <div className="flex-1 overflow-y-auto space-y-3 pr-1">
        {failures.map((f) => {
          const Icon = f.icon;
          const isCurrent = activeFailure === f.id;
          return (
            <button
              key={f.id}
              onClick={() => handleInject(f.id)}
              disabled={!isSimulating}
              className={`w-full text-left p-3 rounded-lg border bg-[#18181b]/40 transition-all duration-200 cursor-pointer flex gap-3 items-start group ${
                !isSimulating 
                  ? "opacity-40 cursor-not-allowed border-[#27272a]/20" 
                  : isCurrent 
                  ? "border-rose-500 bg-rose-500/10 shadow-lg shadow-rose-950/20" 
                  : f.color
              }`}
            >
              <div className="p-2 rounded bg-zinc-800/60 border border-[#27272a] shrink-0 mt-0.5">
                <Icon className="h-4.5 w-4.5" />
              </div>
              <div className="flex-1">
                <span className="text-xs font-bold font-mono block text-zinc-200 group-hover:text-white transition-colors">
                  {f.label}
                </span>
                <span className="text-[10px] text-zinc-500 font-mono block mt-0.5 leading-relaxed">
                  {f.desc}
                </span>
              </div>
            </button>
          );
        })}
      </div>

      {/* Control Buttons */}
      <div className="border-t border-[#27272a]/30 pt-4 mt-4 grid grid-cols-2 gap-3">
        <button
          onClick={onResetSimulator}
          className="flex items-center justify-center gap-2 border border-[#27272a] hover:border-zinc-700 bg-[#18181b] hover:bg-zinc-800 text-xs font-mono py-2.5 rounded-lg text-zinc-300 transition-all cursor-pointer"
        >
          <RotateCcw className="h-3.5 w-3.5" />
          <span>Reset Sim</span>
        </button>
        
        <button
          className="flex items-center justify-center gap-2 border border-[#27272a] bg-[#18181b]/20 cursor-not-allowed text-xs font-mono py-2.5 rounded-lg text-zinc-600 transition-all"
          disabled
        >
          <CirclePlay className="h-3.5 w-3.5" />
          <span>Auto-Cycle</span>
        </button>
      </div>
    </div>
  );
})

export default SimControl;
