/**
 * Files a meeting into a folder from anywhere (a drop, a menu, the meeting
 * header) and handles what follows: a confirmation, and Granola's "Auto-add
 * future meetings in this series?" offer when the meeting is recurring.
 */
import { toast } from 'sonner';

import { foldersCopy } from '../copy';
import { useAddFolderRule, useAddMeetingToFolder, useRemoveMeetingFromFolder } from '../mutations';
import type { Folder, MeetingId, SeriesId } from '../types';

/** How long the series offer stays up: long enough to read and decide. */
const OFFER_DURATION_MS = 12_000;

function reportFailure(error: Error) {
  toast.error(foldersCopy.actionFailed, { description: error.message });
}

/** Offers to collect the meeting's series into `folder`. */
function useSeriesOffer() {
  const addRule = useAddFolderRule();
  return (folder: Folder, seriesId: SeriesId) => {
    toast(foldersCopy.seriesOffer, {
      description: foldersCopy.seriesOfferDescription(folder.name),
      duration: OFFER_DURATION_MS,
      action: {
        label: foldersCopy.seriesAccept,
        onClick: () => {
          addRule.mutate(
            { folderId: folder.id, rule: { kind: 'series', seriesId } },
            { onError: reportFailure },
          );
        },
      },
    });
  };
}

/** Files and unfiles meetings, with the confirmations and the series offer. */
export function useFileMeeting() {
  const add = useAddMeetingToFolder();
  const remove = useRemoveMeetingFromFolder();
  const offerSeries = useSeriesOffer();
  return {
    file: (folder: Folder, meetingId: MeetingId) => {
      add.mutate(
        { folderId: folder.id, meetingId },
        {
          // The folder chip and the sidebar count show the filing (UX-32); only
          // the offer to collect the series needs a toast.
          onSuccess: ({ seriesOffer }) => {
            if (seriesOffer !== null) offerSeries(folder, seriesOffer);
          },
          onError: reportFailure,
        },
      );
    },
    unfile: (folder: Folder, meetingId: MeetingId) => {
      remove.mutate(
        { folderId: folder.id, meetingId },
        {
          onSuccess: ({ seriesRuleRemoved }) => {
            if (seriesRuleRemoved) toast(foldersCopy.seriesRemoved);
          },
          onError: reportFailure,
        },
      );
    },
  };
}
