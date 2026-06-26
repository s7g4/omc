"use client";

import React, { useState } from "react";
import { Radio, Shield, KeyRound, User, ChevronRight, AlertCircle, Loader2 } from "lucide-react";

interface AuthScreenProps {
  onAuthSuccess: (token: string, username: string) => void;
}

export default function AuthScreen({ onAuthSuccess }: AuthScreenProps) {
  const [isLogin, setIsLogin] = useState(true);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username.trim() || !password.trim()) return;

    setError(null);
    setLoading(true);

    const endpoint = isLogin ? "/api/v1/auth/login" : "/api/v1/auth/register";
    
    try {
      // Connect to the Axum backend on port 8081
      const response = await fetch(`http://localhost:8081${endpoint}`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ username, password }),
      });

      const data = await response.json();

      if (!response.ok) {
        throw new Error(data.message || (isLogin ? "Authentication failed" : "Registration failed"));
      }

      onAuthSuccess(data.token, data.username);
    } catch (err: any) {
      setError(err.message || "Failed to connect to authentication server.");
    } finally {
      setLoading(false);
    }
  };

  const handleMockBypass = () => {
    onAuthSuccess("mock-jwt-token-12345", "Guest-Operator");
  };

  return (
    <div className="min-h-screen w-full flex items-center justify-center bg-[#09090b] relative px-4 overflow-hidden grid-bg">
      {/* Decorative Glowing Elements */}
      <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-indigo-500/10 rounded-full blur-3xl pointer-events-none" />
      <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-purple-500/10 rounded-full blur-3xl pointer-events-none" />

      {/* Main card */}
      <div className="w-full max-w-md glass-panel-glow border-[#27272a]/60 rounded-2xl p-8 relative z-10 transition-all duration-300">
        {/* Brand/Logo */}
        <div className="flex flex-col items-center mb-8">
          <div className="bg-indigo-600 p-3 rounded-2xl text-white shadow-xl shadow-indigo-500/20 ring-1 ring-indigo-400 mb-3">
            <Radio className="h-7 w-7 animate-pulse" />
          </div>
          <h1 className="font-extrabold text-lg tracking-wider text-white">OPEN MISSION CONTROL</h1>
          <p className="text-[10px] text-zinc-500 font-mono tracking-widest uppercase mt-0.5">Authorization Portal</p>
        </div>

        {error && (
          <div className="mb-5 p-3.5 bg-rose-500/10 border border-rose-500/25 text-rose-400 rounded-lg flex items-start gap-2 text-xs font-mono">
            <AlertCircle className="h-4 w-4 shrink-0 mt-0.5" />
            <span>{error}</span>
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-1.5">
            <label className="text-[10px] font-mono text-zinc-400 block uppercase tracking-wider">Username</label>
            <div className="relative">
              <User className="absolute left-3 top-3 h-4 w-4 text-zinc-600" />
              <input
                type="text"
                required
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="Operator identifier..."
                className="w-full bg-[#18181b] border border-[#27272a] focus:border-zinc-700 text-xs text-zinc-200 rounded-lg pl-10 pr-3 py-3 outline-none font-mono placeholder-zinc-700"
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <label className="text-[10px] font-mono text-zinc-400 block uppercase tracking-wider">Access Token / Password</label>
            <div className="relative">
              <KeyRound className="absolute left-3 top-3 h-4 w-4 text-zinc-600" />
              <input
                type="password"
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="••••••••••••"
                className="w-full bg-[#18181b] border border-[#27272a] focus:border-zinc-700 text-xs text-zinc-200 rounded-lg pl-10 pr-3 py-3 outline-none font-mono placeholder-zinc-700"
              />
            </div>
          </div>

          <button
            type="submit"
            disabled={loading}
            className="w-full bg-indigo-600 hover:bg-indigo-500 text-white font-mono text-xs py-3 rounded-lg flex items-center justify-center gap-2 border border-indigo-400/20 shadow-lg shadow-indigo-600/10 transition-all font-semibold cursor-pointer select-none"
          >
            {loading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <>
                <span>{isLogin ? "Authenticate Operator" : "Register Command Profile"}</span>
                <ChevronRight className="h-4 w-4" />
              </>
            )}
          </button>
        </form>

        <div className="mt-6 border-t border-[#27272a]/30 pt-4 flex flex-col items-center gap-3">
          <button
            onClick={() => setIsLogin(!isLogin)}
            className="text-xs font-mono text-zinc-400 hover:text-white transition-colors cursor-pointer"
          >
            {isLogin ? "> Request new command registry" : "> Return to operator log in"}
          </button>

          <button
            onClick={handleMockBypass}
            className="text-[10px] font-mono text-zinc-600 hover:text-indigo-400 transition-colors flex items-center gap-1.5 cursor-pointer mt-2"
          >
            <Shield className="h-3 w-3" />
            <span>Emergency offline bypass (Guest Mode)</span>
          </button>
        </div>
      </div>
    </div>
  );
}
