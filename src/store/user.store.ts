import { create } from "zustand";

interface UserStore {
  user: any;
  retry: boolean;
  setUser: (user: any) => void;
  setRetry: (retry: boolean) => void;
}

export const useUserStore = create<UserStore>((set) => ({
  user: null,
  retry: false,
  setUser: (user) => set({ user }),
  setRetry: (retry) => set({ retry }),
}));
