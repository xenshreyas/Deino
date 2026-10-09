import { copy } from '../copy';

/** The NVIDIA Open Model License agreement. */
const LICENCE_URL =
  'https://www.nvidia.com/en-us/agreements/enterprise-software/nvidia-open-model-license/';

/**
 * The notice the Parakeet Unified weights require, for the About screen.
 * The GGUF files are handy-computer's conversion of NVIDIA's model.
 */
export function ModelLicenceNotice() {
  return (
    <section aria-label={copy.noticeTitle} className="space-y-1 text-xs text-fg-muted">
      <h2 className="font-semibold">{copy.noticeTitle}</h2>
      <p>
        {copy.noticeModel} {copy.licence}{' '}
        <a className="underline" href={LICENCE_URL} target="_blank" rel="noreferrer">
          {copy.licenceLink}
        </a>
      </p>
    </section>
  );
}
