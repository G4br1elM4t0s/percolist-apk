import { useState, useRef, useEffect, useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

export type SessionStatus = "pending" | "running" | "paused" | "completed";

export interface Session {
  id: number;
  task_id: number;
  session_number: number;
  session_type: string;
  duration_seconds: number;
  remaining_seconds: number;
  status: SessionStatus;
  created_at: string;
}

const sortBySessionNumber = (arr: Session[]) =>
  [...arr].sort((a, b) => a.session_number - b.session_number);

// Busca a próxima sessão na sequência: primeiro paused, depois pending
const findNextAvailableSession = (list: Session[]) => {
  const sorted = sortBySessionNumber(list);

  // Primeiro procura por sessões pausadas na ordem do session_number
  const pausedIdx = sorted.findIndex(s =>
    s.status === "paused" && s.remaining_seconds > 0
  );

  if (pausedIdx !== -1) {
    return list.findIndex(s => s.id === sorted[pausedIdx].id);
  }

  // Se não encontrou paused, procura por pending na ordem do session_number
  const pendingIdx = sorted.findIndex(s =>
    s.status === "pending" && s.duration_seconds > 0 && s.remaining_seconds > 0
  );

  if (pendingIdx !== -1) {
    return list.findIndex(s => s.id === sorted[pendingIdx].id);
  }

  return -1;
};

const clampSingleRunningIndices = (list: Session[]) => {
  const runnings = list
    .map((s, i) => ({ s, i }))
    .filter(x => x.s.status === "running")
    .sort((a, b) => a.s.session_number - b.s.session_number);
  if (runnings.length <= 1) return { keep: runnings[0]?.i ?? -1, drop: [] };
  const keep = runnings[0].i;
  const drop = runnings.slice(1).map(x => x.i);
  return { keep, drop };
};

// saneia payloads impossíveis vindos do backend
function sanitizePayload(list: Session[]): Session[] {
  // Primeiro, ordena as sessões por session_number
  const sortedList = [...list].sort((a, b) => a.session_number - b.session_number);
  
  return list.map((s, index, arr) => {
    // Garante que remaining_seconds nunca seja maior que duration_seconds
    let remaining = Math.min(s.remaining_seconds, s.duration_seconds);
    
    // Se remaining for negativo, corrige baseado no status
    if (remaining < 0) {
      remaining = s.status === "pending" ? s.duration_seconds : 0;
    }

    // Encontra a posição desta sessão na lista ordenada
    const currentSessionIndex = sortedList.findIndex(session => session.id === s.id);
    
    // Corrige sessões "running" com remaining_seconds = 0
    if (s.status === "running" && remaining <= 0) {
      return { ...s, status: "completed", remaining_seconds: 0 };
    }
    
    // Corrige sessões "paused" com remaining_seconds = 0
    if (s.status === "paused" && remaining <= 0) {
      return { ...s, status: "completed", remaining_seconds: 0 };
    }
    
    // Se a sessão está "completed" mas tem remaining_seconds > 0, corrige
    if (s.status === "completed" && remaining > 0) {
      return { ...s, remaining_seconds: 0 };
    }

    // Verifica se todas as sessões anteriores estão completadas
    const allEarlierSessionsCompleted = sortedList.slice(0, currentSessionIndex).every(
      session => session.status === "completed" || (session.status === "running" && session.remaining_seconds <= 0) || (session.status === "paused" && session.remaining_seconds <= 0)
    );

    // Se nem todas as sessões anteriores estão completadas e esta está "paused" ou "running", deve ser "pending"
    if (!allEarlierSessionsCompleted && (s.status === "paused" || s.status === "running")) {
      return { 
        ...s, 
        status: "pending", 
        remaining_seconds: s.duration_seconds 
      };
    }

    // Encontra a primeira sessão "running" (se houver) após correções
    const runningIndex = sortedList.findIndex(session => {
      if (session.id === s.id && s.status === "running" && remaining > 0) return true;
      if (session.id !== s.id && session.status === "running" && session.remaining_seconds > 0) return true;
      return false;
    });
    
    // Se há uma sessão "running" e esta sessão vem depois dela, deve ser "pending"
    if (runningIndex !== -1 && currentSessionIndex > runningIndex && (s.status === "paused" || s.status === "running")) {
      return { 
        ...s, 
        status: "pending", 
        remaining_seconds: s.duration_seconds 
      };
    }
    
    // Se há múltiplas sessões "running", apenas a primeira deve permanecer "running"
    if (s.status === "running" && remaining > 0) {
      const earlierRunningExists = sortedList.slice(0, currentSessionIndex).some(
        session => session.status === "running" && session.remaining_seconds > 0
      );
      
      if (earlierRunningExists) {
        return { 
          ...s, 
          status: "pending", 
          remaining_seconds: s.duration_seconds 
        };
      }
    }

    // Se chegou até aqui e é "pending" com remaining <= 0, corrige
    if (s.status === "pending" && remaining <= 0) {
      return { ...s, remaining_seconds: s.duration_seconds };
    }
    
    // Retorna com remaining_seconds corrigido
    return { ...s, remaining_seconds: remaining };
  });
}

export function usePomodoroFlow(initial: Session[], taskId: string | number | null | undefined) {
  console.log(initial)
  const [sessions, setSessions] = useState<Session[]>(() => sortBySessionNumber(initial || []));
  const [runningIdx, setRunningIdx] = useState(-1);
  const [pausedIdx, setPausedIdx] = useState(-1);

  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const runningIdxRef = useRef<number>(-1);
  const promotingRef = useRef<boolean>(false);
  const suppressAutostartOnceRef = useRef<boolean>(false);

  const baseRemainingRef = useRef<number>(0);
  const lastSyncAtRef = useRef<number>(Date.now());
  const prevTaskIdRef = useRef<string | number | null>(null);

  const queryClient = useQueryClient();

  const normalizeRemaining = (s: Session) => {
    const remaining = s.remaining_seconds > 0 ? s.remaining_seconds : s.status === "pending" ? s.duration_seconds : 0;
    // Garante que remaining nunca seja maior que duration
    return Math.min(remaining, s.duration_seconds);
  };

  useEffect(() => {
    runningIdxRef.current = runningIdx;
  }, [runningIdx]);

  // ---- Helper: liga imediatamente a melhor sessão (paused>pending na sequência)
  const autostartBestNow = useCallback(async () => {
    if (!taskId) return;
    if (promotingRef.current) return;
    promotingRef.current = true;

    let toPersist: { id: number; remaining: number } | null = null;

    setSessions(prev => {
      if (prev.length === 0) {
        promotingRef.current = false;
        return prev;
      }
      if (prev.some(s => s.status === "running")) {
        promotingRef.current = false;
        return prev;
      }

      const list = [...prev];
      const idx = findNextAvailableSession(list);

      if (idx === -1) {
        promotingRef.current = false;
        return prev;
      }

      const t = list[idx];
      const base = t.status === "pending"
        ? normalizeRemaining(t)
        : t.remaining_seconds > 0
        ? Math.min(t.remaining_seconds, t.duration_seconds) // Garante que não exceda duration
        : 0;

      if (base <= 0) {
        promotingRef.current = false;
        return prev;
      }

      list[idx] = { ...t, status: "running", remaining_seconds: base };

      baseRemainingRef.current = base;
      lastSyncAtRef.current = Date.now();
      setRunningIdx(idx);
      setPausedIdx(-1);

      toPersist = { id: t.id, remaining: base };
      return list;
    });

    try {
      if (toPersist) {
        await invoke("update_pomodoro_session", {
          session_id: toPersist.id,
          remaining_seconds: toPersist.remaining,
          status: "running",
        });
        queryClient.invalidateQueries({ queryKey: ["active-pomodoro-sessions", taskId] });
      }
    } finally {
      promotingRef.current = false;
    }
  }, [taskId, queryClient]);

  // aplica "initial" (da MESMA task), saneia e já liga a melhor sessão
  useEffect(() => {
    if (!taskId) return;
    const raw = sortBySessionNumber(initial || []);
    if (raw.length === 0) return;
    const belongs = raw.every(s => s.task_id === Number(taskId));
    if (!belongs) return;

    // IMPORTANTE: Sempre sanitiza os dados vindos do backend
    const sanitized = sanitizePayload(raw);
    const ordered = sortBySessionNumber(sanitized);
    const { keep, drop } = clampSingleRunningIndices(ordered);

    // Força sessões extras "running" para "pending"
    drop.forEach(i => (ordered[i] = { ...ordered[i], status: "pending", remaining_seconds: ordered[i].duration_seconds }));

    setSessions(ordered);
    setRunningIdx(keep);
    setPausedIdx(ordered.findIndex(s => s.status === "paused"));

    // Inicia a melhor sessão disponível
    queueMicrotask(() => autostartBestNow());
  }, [taskId, initial, autostartBestNow]);

  // ENFORCER
  useEffect(() => {
    if (!taskId) return;
    const ordered = sortBySessionNumber(sessions);
    const { keep, drop } = clampSingleRunningIndices(ordered);
    if (drop.length === 0) return;

    setSessions(prev => {
      const list = [...prev];
      drop.forEach(i => (list[i] = { ...list[i], status: "pending" }));
      return list;
    });

    Promise.all(
      drop.map(i =>
        invoke("update_pomodoro_session", {
          session_id: ordered[i].id,
          remaining_seconds: ordered[i].remaining_seconds,
          status: "pending",
        }).catch(console.error)
      )
    ).finally(() => {
      if (keep !== -1) setRunningIdx(keep);
      queryClient.invalidateQueries({ queryKey: ["active-pomodoro-sessions", taskId] });
    });
  }, [sessions, queryClient, taskId]);

  const clearTicking = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
  }, []);

  // trocar de task: pausar sessão atual + refetch + ligar próxima sessão disponível
  useEffect(() => {
    if (!taskId) return;
    if (prevTaskIdRef.current === null) {
      prevTaskIdRef.current = taskId as any;
      return;
    }
    if (prevTaskIdRef.current === taskId) return;

    const newTaskId = taskId;

    const doSwitch = async () => {
      // Pausa a sessão atual se estiver rodando
      if (runningIdxRef.current !== -1) {
        const idx = runningIdxRef.current;
        const s = sessions[idx];
        if (s) {
          const elapsed = Math.floor((Date.now() - lastSyncAtRef.current) / 1000);
          const remaining = Math.max(0, baseRemainingRef.current - elapsed);

          setSessions(prev => {
            const list = [...prev];
            list[idx] = { ...list[idx], status: "paused", remaining_seconds: remaining };
            return list;
          });

          try {
            await invoke("update_pomodoro_session", {
              session_id: s.id,
              remaining_seconds: remaining,
              status: "paused",
            });
          } catch (e) {
            console.error(e);
          }
        }
      }

      clearTicking();
      setRunningIdx(-1);
      setPausedIdx(-1);
      suppressAutostartOnceRef.current = false;

      // Refetch das sessões da nova task
      await queryClient.refetchQueries({
        queryKey: ["active-pomodoro-sessions", newTaskId],
        exact: true
      });

      prevTaskIdRef.current = newTaskId as any;

      // Inicia a próxima sessão disponível da nova task
      queueMicrotask(() => autostartBestNow());
    };

    void doSwitch();
  }, [taskId, clearTicking, queryClient, sessions, autostartBestNow]);

  // TIMER (sem drift) — só arma se houver tempo
  useEffect(() => {
    if (runningIdx === -1) {
      clearTicking();
      return;
    }
    if (intervalRef.current) return;

    const s = sessions[runningIdx];
    const startRemaining =
      s?.status === "pending" ? normalizeRemaining(s) : s ? (s.remaining_seconds > 0 ? s.remaining_seconds : 0) : 0;

    if (!startRemaining || startRemaining <= 0) {
      clearTicking();
      setRunningIdx(-1);
      return;
    }

    baseRemainingRef.current = startRemaining;
    lastSyncAtRef.current = Date.now();

    intervalRef.current = setInterval(() => {
      setSessions(prev => {
        const idx = runningIdxRef.current;
        if (idx === -1) return prev;

        const list = [...prev];
        const cur = list[idx];
        if (!cur) return prev;

        const elapsed = Math.floor((Date.now() - lastSyncAtRef.current) / 1000);
        const remaining = Math.max(0, baseRemainingRef.current - elapsed);

        list[idx] = { ...cur, remaining_seconds: remaining };

        if (remaining === 0) {
          list[idx] = { ...list[idx], status: "completed", remaining_seconds: 0 };

          // Busca a próxima sessão disponível na sequência
          const nextIdx = findNextAvailableSession(list);

          if (nextIdx !== -1 && !promotingRef.current) {
            promotingRef.current = true;

            const next = list[nextIdx];
            const nextBase = next.status === "pending"
              ? normalizeRemaining(next)
              : next.remaining_seconds > 0
              ? next.remaining_seconds
              : 0;

            if (nextBase > 0) {
              list[nextIdx] = { ...next, status: "running", remaining_seconds: nextBase };
              setRunningIdx(nextIdx);
              setPausedIdx(-1);

              baseRemainingRef.current = nextBase;
              lastSyncAtRef.current = Date.now();

              // Atualiza a sessão completada
              invoke("update_pomodoro_session", {
                session_id: cur.id,
                remaining_seconds: 0,
                status: "completed",
              }).catch(console.error);

              // Inicia a próxima sessão
              invoke("update_pomodoro_session", {
                session_id: list[nextIdx].id,
                remaining_seconds: nextBase,
                status: "running",
              })
                .catch(console.error)
                .finally(() => {
                  promotingRef.current = false;
                  queryClient.invalidateQueries({ queryKey: ["active-pomodoro-sessions", taskId] });
                });
            } else {
              promotingRef.current = false;
            }
          } else {
            setRunningIdx(-1);
            setPausedIdx(-1);
            invoke("update_pomodoro_session", {
              session_id: cur.id,
              remaining_seconds: 0,
              status: "completed",
            }).catch(console.error);
          }
        }

        return list;
      });
    }, 1000);

    return () => clearTicking();
  }, [runningIdx, sessions, clearTicking, queryClient, taskId]);

  // SYNC leve
  useEffect(() => {
    if (runningIdx === -1) return;
    const id = sessions[runningIdx]?.id;
    if (!id) return;

    const t = setInterval(() => {
      const idx = runningIdxRef.current;
      const s = sessions[idx];
      if (!s) return;

      invoke("update_pomodoro_session", {
        session_id: s.id,
        remaining_seconds: s.remaining_seconds,
        status: s.status,
      }).catch(console.error);
    }, 5000);

    return () => clearInterval(t);
  }, [runningIdx, sessions[runningIdx]?.id, sessions]);

  // START manual
  const start = useCallback(async (sessionIndex?: number) => {
    if (!taskId) return;
    await autostartBestNow(); // delega pra mesma rotina
  }, [taskId, autostartBestNow]);

  const pause = useCallback(async () => {
    if (runningIdx === -1) return;

    let persistedRemaining = 0;
    let persistedId: number | null = null;

    setSessions(prev => {
      const idx = runningIdxRef.current;
      if (idx === -1) return prev;
      const list = [...prev];
      const s = list[idx];
      if (!s) return prev;

      const elapsed = Math.floor((Date.now() - lastSyncAtRef.current) / 1000);
      const remaining = Math.max(0, baseRemainingRef.current - elapsed);

      persistedRemaining = remaining;
      persistedId = s.id;

      list[idx] = { ...s, status: "paused", remaining_seconds: remaining };
      setPausedIdx(idx);
      setRunningIdx(-1);
      return list;
    });

    clearTicking();

    try {
      if (persistedId != null && taskId) {
        await invoke("update_pomodoro_session", {
          session_id: persistedId,
          remaining_seconds: persistedRemaining,
          status: "paused",
        });
        queryClient.invalidateQueries({ queryKey: ["active-pomodoro-sessions", taskId] });
      }
    } catch (e) {
      console.error(e);
    }
  }, [runningIdx, clearTicking, queryClient, taskId]);

  const resume = useCallback(async () => {
    if (!taskId) return;
    await autostartBestNow(); // mesma regra de "sempre ligar quando voltar"
  }, [taskId, autostartBestNow]);

  const resetAll = useCallback(async () => {
    await autostartBestNow();
  }, [autostartBestNow]);

  const currentSession =
    (runningIdx !== -1 && sessions[runningIdx]) ||
    (pausedIdx !== -1 && sessions[pausedIdx]) ||
    sessions.find(s => s.status === "pending") ||
    null;

  return {
    sessions,
    currentSession,
    runningIdx,
    pausedIdx,
    start,
    pause,
    resume,
    resetAll,
  };
}
