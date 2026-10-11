import React, { createContext, useContext, useState, useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useLocation } from '@tanstack/react-router';
import { meetingKeys, useMeetings, type Meeting } from '@/features/meetings';

/** Stable empty list, so consumers do not re-render while meetings load. */
const NO_MEETINGS: Meeting[] = [];

export interface CurrentMeeting {
  id: string;
  title: string;
}

interface SidebarContextType {
  currentMeeting: CurrentMeeting | null;
  setCurrentMeeting: (meeting: CurrentMeeting | null) => void;
  /** Every meeting, newest first, from the `meetings` plugin (kept fresh by DbChanged). */
  meetings: Meeting[];
  isMeetingActive: boolean;
  setIsMeetingActive: (active: boolean) => void;
  /** Refetches the meeting list now. DbChanged already does this after every write. */
  refetchMeetings: () => Promise<void>;

}

const SidebarContext = createContext<SidebarContextType | null>(null);

export const useSidebar = () => {
  const context = useContext(SidebarContext);
  if (!context) {
    throw new Error('useSidebar must be used within a SidebarProvider');
  }
  return context;
};

export function SidebarProvider({ children }: { children: React.ReactNode }) {
  const [currentMeeting, setCurrentMeeting] = useState<CurrentMeeting | null>({ id: 'intro-call', title: '+ New Call' });
  const { data: meetings = NO_MEETINGS } = useMeetings();
  const queryClient = useQueryClient();
  const [isMeetingActive, setIsMeetingActive] = useState(false);

  const pathname = useLocation({ select: (location) => location.pathname });

  const refetchMeetings = React.useCallback(
    () => queryClient.invalidateQueries({ queryKey: meetingKeys.all }),
    [queryClient],
  );

  // Update current meeting when on home page
  useEffect(() => {
    if (pathname === '/') {
      setCurrentMeeting({ id: 'intro-call', title: '+ New Call' });
    }
  }, [pathname]);

  return (
    <SidebarContext.Provider value={{
      currentMeeting,
      setCurrentMeeting,
      meetings,
      isMeetingActive,
      setIsMeetingActive,
      refetchMeetings,

    }}>
      {children}
    </SidebarContext.Provider>
  );
}
