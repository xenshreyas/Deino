# deino-denoise

## Role

The voice check behind the microphone track's last transcription step:
[DeepFilterNet3](https://github.com/Rikorose/DeepFilterNet) (MIT OR
Apache-2.0, run on [tract](https://github.com/sonos/tract)) cleans a short
stretch of the echo-cancelled microphone audio, and `Cleaner::cleaned_peak`
says how loud the cleaned audio gets under a transcript line. Keyboard
clicks and other noise clean to near silence; a voice does not. Whisper
invents text ("Thank you.", whole sentences) for such noise, and
`deino-recording`'s offline transcriber drops a line of the user's when the
cleaned audio has no voice under it and the raw audio is below speaking
level.

## Owns

- `Cleaner`: one loaded model, run from a few seconds before a stretch so
  its state has settled, at the CLI's thresholds the A/B test used.
- `peak_db`: the loudest 100 ms frame of 16 kHz audio over a span, in dBFS
  (frames on the track's 100 ms grid).

## Does Not Own

- Which lines to drop, or the thresholds (`deino-recording::offline`).
- Cleaning audio for listening or for the transcriber: transcribing
  cleaned audio cut real speech in the A/B test (Oct 2026), so the model
  only ever judges.

## Invariants

- Input is 16 kHz mono; the model runs at its own 48 kHz.
- No network, no files: the model is embedded at build time.
