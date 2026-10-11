/**
 * Voice profiles (CAP-04): everyone whose voice Deino recognises, their
 * samples, and deleting a sample or forgetting a voice. A standalone
 * section, so Settings → People shows it today and ORG-04's person page can
 * embed it later.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { Accordion } from '@/ui/accordion';
import { Button } from '@/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';
import { SkeletonText } from '@/ui/skeleton';

import { speakersCopy } from '../copy';
import { useDeleteVoiceprint, useForgetVoice, useVoiceProfiles } from '../queries';
import type { VoiceProfile } from '../types';
import { VoiceProfileItem } from './voice-profile-item';

/** Forgets `profile`'s voice, then closes. */
function useForget(profile: VoiceProfile | null, onClose: () => void) {
  const forget = useForgetVoice();
  const confirm = () => {
    if (profile === null) return;
    forget.mutate(profile.personId, {
      onSuccess: () => toast.success(speakersCopy.forgotten),
      onError: (error) => toast.error(speakersCopy.deleteFailed, { description: error.message }),
      onSettled: onClose,
    });
  };
  return { confirm, pending: forget.isPending };
}

/** Asks before forgetting a whole voice. */
function ForgetVoiceDialog({
  profile,
  onClose,
}: {
  profile: VoiceProfile | null;
  onClose: () => void;
}) {
  const { confirm, pending } = useForget(profile, onClose);
  const name = profile?.name ?? '';
  return (
    <Dialog
      open={profile !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{speakersCopy.forgetTitle(name)}</DialogTitle>
          <DialogDescription>{speakersCopy.forgetBody}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="secondary" onClick={onClose}>
            {speakersCopy.cancel}
          </Button>
          <Button variant="danger-solid" disabled={pending} onClick={confirm}>
            {speakersCopy.forgetConfirm}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function ProfilesEmpty() {
  return (
    <div className="rounded-lg border border-border px-4 py-5 text-center">
      <p className="text-sm font-semibold text-fg">{speakersCopy.profilesEmptyTitle}</p>
      <p className="mt-1 text-xs text-pretty text-fg-muted">{speakersCopy.profilesEmptyBody}</p>
    </div>
  );
}

/** The list of voice profiles, or its resting, loading or error state. */
export function VoiceProfiles() {
  const profiles = useVoiceProfiles();
  const deleteSample = useDeleteVoiceprint();
  const [forgetting, setForgetting] = useState<VoiceProfile | null>(null);
  if (profiles.isPending) return <SkeletonText lines={3} />;
  if (profiles.isError)
    return <p className="text-sm text-danger">{speakersCopy.profilesLoadError}</p>;
  if (profiles.data.length === 0) return <ProfilesEmpty />;
  return (
    <>
      <Accordion type="multiple" className="rounded-lg border border-border bg-surface">
        {profiles.data.map((profile) => (
          <VoiceProfileItem
            key={profile.personId}
            profile={profile}
            onForget={setForgetting}
            onDeleteSample={(id) => {
              deleteSample.mutate(id, {
                onError: (error) =>
                  toast.error(speakersCopy.deleteFailed, { description: error.message }),
              });
            }}
          />
        ))}
      </Accordion>
      <ForgetVoiceDialog
        profile={forgetting}
        onClose={() => {
          setForgetting(null);
        }}
      />
    </>
  );
}
