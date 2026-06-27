"use client";

import React, { useState } from "react";
import { 
  FolderKanban, 
  Plus, 
  Satellite, 
  Calendar, 
  Trash2 
} from "lucide-react";

export interface Mission {
  id: string;
  name: string;
  description: string;
  status: "planned" | "active" | "completed" | "aborted";
  satellite_id: string | null;
  start_date: string;
}

interface MissionsOverviewProps {
  missions: Mission[];
  satellites: Array<{ id: string; name: string }>;
  onCreateMission: (name: string, description: string, satelliteId: string | null) => void;
  onDeleteMission: (id: string) => void;
}

export default function MissionsOverview({ 
  missions, 
  satellites,
  onCreateMission,
  onDeleteMission 
}: MissionsOverviewProps) {
  const [showAddForm, setShowAddForm] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [satelliteId, setSatelliteId] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    onCreateMission(name, description, satelliteId || null);
    setName("");
    setDescription("");
    setSatelliteId("");
    setShowAddForm(false);
  };

  const getStatusBadge = (status: Mission["status"]) => {
    const styles = {
      planned: "bg-blue-500/10 text-blue-400 border-blue-500/20",
      active: "bg-emerald-500/10 text-emerald-400 border-emerald-500/20 pulse-green",
      completed: "bg-zinc-500/10 text-zinc-400 border-zinc-500/20",
      aborted: "bg-rose-500/10 text-rose-400 border-rose-500/20",
    };
    return (
      <span className={`px-2 py-0.5 rounded border text-[9px] font-bold font-mono uppercase tracking-wider ${styles[status]}`}>
        {status}
      </span>
    );
  };

  return (
    <div className="space-y-6">
      {/* Top action row */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-md font-bold text-white font-mono uppercase tracking-wider">Mission Directory</h3>
          <p className="text-[11px] text-zinc-500 font-mono">Plan and assign satellites to operational missions</p>
        </div>

        <button
          onClick={() => setShowAddForm(!showAddForm)}
          className="flex items-center gap-1.5 bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-mono px-3 py-2 rounded-lg shadow-lg shadow-indigo-600/20 transition-all font-medium border border-indigo-400/20 cursor-pointer"
        >
          <Plus className="h-3.5 w-3.5" />
          <span>New Mission</span>
        </button>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 items-start">
        {/* Mission List */}
        <div className="lg:col-span-2 space-y-4">
          {missions.length === 0 ? (
            <div className="glass-panel border-[#27272a]/50 rounded-xl p-8 text-center">
              <FolderKanban className="h-10 w-10 text-zinc-600 mx-auto mb-3" />
              <h4 className="text-sm font-bold text-zinc-400 font-mono">No Active Missions</h4>
              <p className="text-xs text-zinc-500 mt-1 max-w-xs mx-auto">Create a new mission profile and delegate satellite payloads to start telemetry logs.</p>
            </div>
          ) : (
            missions.map((mission) => {
              const assignedSat = satellites.find(s => s.id === mission.satellite_id);
              return (
                <div 
                  key={mission.id}
                  className="glass-panel border-[#27272a]/50 hover:border-zinc-700/80 rounded-xl p-5 transition-all duration-200"
                >
                  <div className="flex justify-between items-start mb-2">
                    <div>
                      <h4 className="font-bold text-sm text-white tracking-wide">{mission.name}</h4>
                      <div className="flex items-center gap-4 mt-1">
                        <div className="flex items-center gap-1 text-[10px] text-zinc-500 font-mono">
                          <Calendar className="h-3 w-3 text-zinc-600" />
                          <span>Started: {mission.start_date}</span>
                        </div>
                        {assignedSat && (
                          <div className="flex items-center gap-1 text-[10px] text-indigo-400 font-mono">
                            <Satellite className="h-3 w-3" />
                            <span>Payload: {assignedSat.name}</span>
                          </div>
                        )}
                      </div>
                    </div>
                    <div className="flex items-center gap-3">
                      {getStatusBadge(mission.status)}
                      <button
                        onClick={() => onDeleteMission(mission.id)}
                        className="p-1 text-zinc-600 hover:text-rose-400 hover:bg-zinc-800/40 rounded transition-colors"
                        title="Delete mission"
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </button>
                    </div>
                  </div>

                  <p className="text-xs text-zinc-400 mt-3 font-mono leading-relaxed bg-[#18181b]/35 border border-[#27272a]/30 p-2.5 rounded-lg">
                    {mission.description}
                  </p>
                </div>
              );
            })
          )}
        </div>

        {/* Add Mission Form Sidebar */}
        {showAddForm ? (
          <div className="glass-panel border-indigo-500/20 bg-indigo-500/[0.01] rounded-xl p-5 space-y-4">
            <h4 className="text-xs font-bold text-white font-mono uppercase tracking-widest border-b border-[#27272a] pb-2">
              Setup Operational Mission
            </h4>
            
            <form onSubmit={handleSubmit} className="space-y-4">
              <div className="space-y-1.5">
                <label className="text-[10px] font-mono text-zinc-400 block uppercase">Mission Code Name</label>
                <input
                  type="text"
                  required
                  placeholder="e.g. Artemis-III Orbital Survey"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="w-full bg-[#18181b] border border-[#27272a] focus:border-zinc-700 text-xs text-zinc-200 rounded-lg p-2.5 outline-none font-mono"
                />
              </div>

              <div className="space-y-1.5">
                <label className="text-[10px] font-mono text-zinc-400 block uppercase">Assign Satellite Payload</label>
                <select
                  value={satelliteId}
                  onChange={(e) => setSatelliteId(e.target.value)}
                  className="w-full bg-[#18181b] border border-[#27272a] focus:border-zinc-700 text-xs text-zinc-300 rounded-lg p-2.5 outline-none font-mono cursor-pointer"
                >
                  <option value="">Unassigned (Standby)</option>
                  {satellites.map((s) => (
                    <option key={s.id} value={s.id}>{s.name}</option>
                  ))}
                </select>
              </div>

              <div className="space-y-1.5">
                <label className="text-[10px] font-mono text-zinc-400 block uppercase">Mission Description</label>
                <textarea
                  placeholder="Describe mission scope, targets, and orbital parameters..."
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  rows={4}
                  className="w-full bg-[#18181b] border border-[#27272a] focus:border-zinc-700 text-xs text-zinc-200 rounded-lg p-2.5 outline-none font-mono resize-none"
                />
              </div>

              <div className="grid grid-cols-2 gap-3 pt-2">
                <button
                  type="button"
                  onClick={() => setShowAddForm(false)}
                  className="border border-[#27272a] hover:border-zinc-700 text-zinc-400 hover:text-zinc-200 text-xs font-mono py-2 rounded-lg transition-colors cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-mono py-2 rounded-lg transition-colors font-medium border border-indigo-400/20 cursor-pointer"
                >
                  Deploy Mission
                </button>
              </div>
            </form>
          </div>
        ) : (
          <div className="glass-panel border-[#27272a]/40 rounded-xl p-5 text-center text-zinc-500 font-mono text-[10px] space-y-2">
            <span>&gt; SELECT &quot;NEW MISSION&quot; TO LAUNCH A TARGET PROFILE</span>
          </div>
        )}
      </div>
    </div>
  );
}
