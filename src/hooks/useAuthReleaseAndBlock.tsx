import { useEffect, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { useUserStore } from "../store";
import { api } from "../lib/api";

export const useAuthReleaseAndBlock = () => {
  const { user, setUser } = useUserStore();

  const [token, setToken] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const retryTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const { mutateAsync: doReleaseAndBlock, isPending } = useMutation({
    mutationFn: async ({ token, sessionId }: { token: string; sessionId: string }) => {
      return await api.post("/auth/sync", { token, sessionId });
    },
    onError: () => {
      if (retryTimeoutRef.current) clearTimeout(retryTimeoutRef.current);
      retryTimeoutRef.current = setTimeout(() => {
        console.warn("⏳ Tentando novamente /auth/sync após 5 minutos...");
        if (token && sessionId) {
          doReleaseAndBlock({ token, sessionId });
        }
      }, 5 * 60 * 1000);
    },
  });

  useEffect(() => {
    const storedToken = localStorage.getItem("auth_token");
    const storedSessionId = localStorage.getItem("auth_session_id");

    if (storedToken && storedSessionId) {
      setToken(storedToken);
      setSessionId(storedSessionId);

      doReleaseAndBlock({ token: storedToken, sessionId: storedSessionId });
    }

    const handleStorage = () => {
      const newToken = localStorage.getItem("auth_token");
      const newSessionId = localStorage.getItem("auth_session_id");
      if (newToken && newSessionId) {
        setToken(newToken);
        setSessionId(newSessionId);
        doReleaseAndBlock({ token: newToken, sessionId: newSessionId });
      }
    };

    window.addEventListener("storage", handleStorage);
    return () => {
      window.removeEventListener("storage", handleStorage);
      if (retryTimeoutRef.current) clearTimeout(retryTimeoutRef.current);
    };
  }, [doReleaseAndBlock]);

  return {
    user,
    setUser,
    token,
    sessionId,
    isPending,
  };
};
