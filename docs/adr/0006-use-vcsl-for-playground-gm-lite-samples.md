# Use VCSL for Playground GM-lite samples

Loom will use VCSL (Versilian Community Sample Library) as the source material for Playground's GM-lite browser preview samples. Playground will not try to ship a complete General MIDI sound set. Instead, it will map General MIDI program numbers and channel 10 percussion notes onto a small built-in sample set copied from VCSL samples, with source file names, license, attribution, and source URL recorded alongside the bundled audio files.

We choose this because Playground should sound better than the current oscillator preview without making Web MIDI, external synthesizers, or large SoundFont downloads part of the beginner path. VCSL is CC0, making it practical to redistribute a small curated subset with the Playground. Strudel (`https://strudel.cc/`) is the main reference for this direction: it demonstrates that a web music playground can provide a good default experience with a built-in sample map, lazy sample loading, and WebAudio playback while still keeping custom or external sound paths possible later.

**Considered Options**

- Web MIDI as the primary playback path: accurate when the user has a configured synth, but too much setup for beginners.
- Full General MIDI SoundFont: closer to GM behavior, but large, slower to load, and more complex to license and integrate.
- Pure WebAudio synthesis: lightweight and license-simple, but acoustic instruments and drums are much harder to make pleasant.
- VCSL-derived GM-lite samples: not GM-complete, but small, redistributable, and good enough for Playground preview.
